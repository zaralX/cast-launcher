pub mod detect;
pub mod runtime;
pub mod select;

use std::path::PathBuf;

use tokio::sync::{Mutex, RwLock};

use crate::config::{AppConfig, JavaMode};
use crate::error::{CommandError, CommandResult};
use crate::mojang::profile::JavaRequirement;
use crate::mojang::rules::RuntimeContext;
use crate::net::download::{DownloadRegistry, ProgressSink};
use crate::net::meta_cache::MetaCache;

use detect::JavaRuntime;

#[derive(Default)]
pub struct JavaRegistry {
    runtimes: RwLock<Option<Vec<JavaRuntime>>>,
    scan_lock: Mutex<()>,
}

impl JavaRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn cached(&self) -> Option<Vec<JavaRuntime>> {
        self.runtimes.read().await.clone()
    }

    pub async fn invalidate(&self) {
        *self.runtimes.write().await = None;
    }

    pub async fn list(&self, runtimes_dir: PathBuf, force: bool) -> CommandResult<Vec<JavaRuntime>> {
        if !force {
            if let Some(cached) = self.cached().await {
                return Ok(cached);
            }
        }

        let _guard = self.scan_lock.lock().await;

        if !force {
            if let Some(cached) = self.cached().await {
                return Ok(cached);
            }
        }

        let found = detect::list(vec![runtimes_dir]).await?;
        *self.runtimes.write().await = Some(found.clone());

        Ok(found)
    }
}

pub struct ResolveOptions<'a> {
    pub allow_download: bool,
    pub job_id: &'a str,
    pub on_progress: Option<ProgressSink>,
}

impl Default for ResolveOptions<'_> {
    fn default() -> Self {
        Self {
            allow_download: false,
            job_id: "java",
            on_progress: None,
        }
    }
}

pub async fn resolve(
    registry: &JavaRegistry,
    downloads: &DownloadRegistry,
    meta: &MetaCache,
    config: &AppConfig,
    runtimes_dir: PathBuf,
    requirement: &JavaRequirement,
    options: ResolveOptions<'_>,
) -> CommandResult<JavaRuntime> {
    if config.java.java_mode == JavaMode::Manual {
        if let Some(path) = config.manual_java_path() {
            return detect::probe(path.to_string()).await?.ok_or_else(|| {
                CommandError::java_not_found(format!("Java по указанному пути не найдена: {path}"))
            });
        }
    }

    let installed = registry.list(runtimes_dir.clone(), false).await?;

    if config.java.java_mode == JavaMode::System {
        return select::pick_system(&installed)
            .cloned()
            .ok_or_else(|| CommandError::java_not_found("В системе не найдено ни одной Java"));
    }

    if let Some(picked) = select::pick(&installed, requirement) {
        return Ok(picked.clone());
    }

    if options.allow_download {
        if let Some(component) = &requirement.component {
            let target = runtimes_dir.join(component);
            let ctx = download_context(&installed);

            let installed_version = runtime::install(
                component,
                &target,
                &ctx,
                meta,
                downloads,
                options.job_id,
                options.on_progress,
            )
            .await?;

            if installed_version.is_some() {
                registry.invalidate().await;
                let rescanned = registry.list(runtimes_dir, true).await?;

                if let Some(picked) = select::pick(&rescanned, requirement) {
                    return Ok(picked.clone());
                }
            }
        }
    }

    Err(CommandError::java_not_found(format!(
        "Для этой сборки нужна {}, {}",
        requirement.describe(),
        select::describe_installed(&installed)
    )))
}

fn download_context(installed: &[JavaRuntime]) -> RuntimeContext {
    match installed.iter().find(|runtime| runtime.is_64bit).or_else(|| installed.first()) {
        Some(runtime) => runtime.runtime_context(),
        None => RuntimeContext::new(std::env::consts::ARCH, ""),
    }
}
