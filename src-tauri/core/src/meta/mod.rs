pub mod fabric;
pub mod forge;
pub mod neoforge;
pub mod vanilla;

use crate::error::CommandResult;
use crate::instance::{Instance, LoaderType};
use crate::mojang::profile::{JavaRequirement, ResolvedProfile};
use crate::mojang::rules::RuntimeContext;
use crate::mojang::version::VersionPackage;
use crate::net::meta_cache::MetaCache;
use crate::paths::LauncherPaths;

pub struct Resolver<'a> {
    pub paths: &'a LauncherPaths,
    pub meta: &'a MetaCache,
}

impl<'a> Resolver<'a> {
    pub fn new(paths: &'a LauncherPaths, meta: &'a MetaCache) -> Self {
        Self { paths, meta }
    }

    pub async fn base_package(&self, instance: &Instance) -> CommandResult<VersionPackage> {
        vanilla::package(self.meta, &instance.minecraft_version).await
    }

    pub fn java_requirement(&self, base: &VersionPackage) -> JavaRequirement {
        JavaRequirement::from_package(base)
    }

    pub async fn profile(
        &self,
        instance: &Instance,
        base: &VersionPackage,
        ctx: &RuntimeContext,
    ) -> CommandResult<ResolvedProfile> {
        match forge::Family::of(instance.loader) {
            Some(family) => {
                let version = instance.require_loader_version()?;
                let installed = forge::installed(self.paths, family, version).await?;

                forge::profile(self.paths, instance, base, &installed, family, ctx)
            }
            None => match instance.loader {
                LoaderType::Fabric => {
                    let loader = self.fabric_loader(instance).await?;
                    fabric::profile(self.paths, instance, base, &loader, ctx)
                }
                _ => vanilla::profile(self.paths, instance, base, ctx),
            },
        }
    }

    pub async fn fabric_loader(&self, instance: &Instance) -> CommandResult<fabric::FabricLoader> {
        fabric::loader(
            self.meta,
            &instance.minecraft_version,
            instance.loader_version.as_deref().unwrap_or("latest"),
        )
        .await
    }
}
