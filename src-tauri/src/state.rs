use std::path::PathBuf;
use std::sync::Arc;

use tauri::{AppHandle, Manager};
use tokio::sync::RwLock;

use cast_core::account::AccountStore;
use cast_core::config::{self, AppConfig};
use cast_core::error::{CommandError, CommandResult};
use cast_core::instance::InstanceRegistry;
use cast_core::java::JavaRegistry;
use cast_core::net::download::DownloadRegistry;
use cast_core::net::meta_cache::MetaCache;
use cast_core::paths::LauncherPaths;

use crate::install::InstallRegistry;
use crate::launch::process::ProcessRegistry;

pub struct AppState {
    config_root: PathBuf,
    config: RwLock<AppConfig>,
    paths: RwLock<LauncherPaths>,
    pub meta: MetaCache,
    pub downloads: DownloadRegistry,
    pub installs: InstallRegistry,
    pub instances: InstanceRegistry,
    pub processes: ProcessRegistry,
    pub java: JavaRegistry,
    pub accounts: AccountStore,
}

impl AppState {
    pub async fn initialize(app: &AppHandle) -> CommandResult<Arc<Self>> {
        let config_root = app
            .path()
            .app_config_dir()
            .map_err(|e| CommandError::fs("Не удалось определить каталог конфигурации").with_details(e.to_string()))?;

        cast_core::fs_util::ensure_dir(&config_root).await?;

        let bootstrap = LauncherPaths::new(config_root.clone(), None);
        let config = config::load(&config_root, &bootstrap.config_file()).await?;
        let paths = LauncherPaths::new(config_root.clone(), Some(&config.launcher.dir));

        let accounts = AccountStore::load(paths.accounts_file()).await;

        let state = Arc::new(Self {
            config_root,
            meta: MetaCache::new(paths.meta_cache()),
            config: RwLock::new(config),
            paths: RwLock::new(paths),
            downloads: DownloadRegistry::new(),
            installs: InstallRegistry::new(),
            instances: InstanceRegistry::new(),
            processes: ProcessRegistry::new(),
            java: JavaRegistry::new(),
            accounts,
        });

        let paths = state.paths().await;
        state.instances.reload(&paths).await?;

        Ok(state)
    }

    pub async fn config(&self) -> AppConfig {
        self.config.read().await.clone()
    }

    pub async fn paths(&self) -> LauncherPaths {
        self.paths.read().await.clone()
    }

    pub async fn update_config(&self, config: AppConfig) -> CommandResult<AppConfig> {
        let file = self.paths.read().await.config_file();
        config::save(&file, &config).await?;

        let updated_paths = LauncherPaths::new(self.config_root.clone(), Some(&config.launcher.dir));

        *self.paths.write().await = updated_paths;
        *self.config.write().await = config.clone();

        self.meta.relocate(self.paths().await.meta_cache()).await;
        self.java.invalidate().await;

        Ok(config)
    }
}
