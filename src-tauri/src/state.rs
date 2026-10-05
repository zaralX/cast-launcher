use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use tauri::{AppHandle, Manager};
use tokio::sync::RwLock;

use cast_core::account::AccountStore;
use cast_core::config::{self, AppConfig};
use cast_core::error::{CommandError, CommandResult};
use cast_core::instance::InstanceRegistry;
use cast_core::java::JavaRegistry;
use cast_core::mods::install::InstallPlan;
use cast_core::mods::updates::ModUpdate;
use cast_core::net::download::DownloadRegistry;
use cast_core::net::meta_cache::MetaCache;
use cast_core::paths::LauncherPaths;

use cast_core::import::ImportRegistry;

use crate::install::blocked::BlockedRegistry;
use crate::install::InstallRegistry;
use crate::launch::process::ProcessRegistry;

pub struct AppState {
    config_root: PathBuf,
    config: RwLock<AppConfig>,
    paths: RwLock<LauncherPaths>,
    pub meta: MetaCache,
    pub downloads: DownloadRegistry,
    pub installs: InstallRegistry,
    pub blocked: BlockedRegistry,
    pub imports: Arc<ImportRegistry>,
    pub instances: InstanceRegistry,
    pub processes: ProcessRegistry,
    pub java: JavaRegistry,
    pub accounts: AccountStore,
    pub mods: ModLocks,
    pub mod_updates: RwLock<HashMap<String, Vec<ModUpdate>>>,
    pub mod_plans: RwLock<HashMap<String, (String, InstallPlan)>>,
}

#[derive(Default)]
pub struct ModLocks(RwLock<HashMap<String, Arc<tokio::sync::Mutex<()>>>>);

impl ModLocks {
    pub async fn of(&self, instance_id: &str) -> Arc<tokio::sync::Mutex<()>> {
        if let Some(lock) = self.0.read().await.get(instance_id) {
            return Arc::clone(lock);
        }

        Arc::clone(
            self.0
                .write()
                .await
                .entry(instance_id.to_string())
                .or_default(),
        )
    }
}

impl AppState {
    pub async fn initialize(app: &AppHandle) -> CommandResult<Arc<Self>> {
        let config_root = app.path().app_config_dir().map_err(|e| {
            CommandError::fs("error.reason.fs.no_config_dir").with_details(e.to_string())
        })?;

        cast_core::fs_util::ensure_dir(&config_root).await?;

        log::info!("Config directory: {}", config_root.display());

        let bootstrap = LauncherPaths::new(config_root.clone(), None);
        let config = config::load(&config_root, &bootstrap.config_file()).await?;
        let paths = LauncherPaths::new(config_root.clone(), Some(&config.launcher.dir));

        log::info!("Launcher data directory: {}", paths.root().display());

        let accounts = AccountStore::load(paths.accounts_file()).await;

        let state = Arc::new(Self {
            config_root,
            meta: MetaCache::new(paths.meta_cache()),
            config: RwLock::new(config),
            paths: RwLock::new(paths),
            downloads: DownloadRegistry::new(),
            installs: InstallRegistry::new(),
            blocked: BlockedRegistry::new(),
            imports: Arc::new(ImportRegistry::new()),
            instances: InstanceRegistry::new(),
            processes: ProcessRegistry::new(),
            java: JavaRegistry::new(),
            accounts,
            mods: ModLocks::default(),
            mod_updates: RwLock::new(HashMap::new()),
            mod_plans: RwLock::new(HashMap::new()),
        });

        let paths = state.paths().await;

        if let Err(error) = state.instances.reload(&paths).await {
            log::error!(
                "Failed to read instances in {}: {error}",
                paths.instances_root().display()
            );
        }

        Ok(state)
    }

    pub async fn config(&self) -> AppConfig {
        self.config.read().await.clone()
    }

    pub async fn paths(&self) -> LauncherPaths {
        self.paths.read().await.clone()
    }

    pub async fn update_config(&self, config: AppConfig) -> CommandResult<bool> {
        let updated_paths =
            LauncherPaths::new(self.config_root.clone(), Some(&config.launcher.dir));
        let relocated = updated_paths.root() != self.paths.read().await.root();

        if relocated {
            cast_core::fs_util::ensure_dir(updated_paths.root()).await?;
        }

        let file = self.paths.read().await.config_file();
        config::save(&file, &config).await?;

        *self.paths.write().await = updated_paths;
        *self.config.write().await = config;

        let paths = self.paths().await;

        self.meta.relocate(paths.meta_cache()).await;
        self.java.invalidate().await;

        if relocated {
            self.accounts.relocate(paths.accounts_file()).await;
            self.instances.reload(&paths).await?;
        }

        Ok(relocated)
    }
}
