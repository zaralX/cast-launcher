use crate::error::{CommandError, CommandResult};
use crate::instance::Instance;
use crate::mojang::profile::{resolve_libraries, JavaRequirement, ResolvedArguments, ResolvedProfile};
use crate::mojang::rules::RuntimeContext;
use crate::mojang::version::{VersionManifest, VersionPackage};
use crate::net::meta_cache::MetaCache;
use crate::paths::LauncherPaths;

pub const VERSION_MANIFEST_URL: &str =
    "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";

pub async fn manifest(meta: &MetaCache) -> CommandResult<VersionManifest> {
    meta.fetch_json(VERSION_MANIFEST_URL).await
}

pub async fn package(meta: &MetaCache, minecraft_version: &str) -> CommandResult<VersionPackage> {
    let manifest = manifest(meta).await?;

    let entry = manifest.find(minecraft_version).ok_or_else(|| {
        CommandError::version_not_found(format!(
            "Версия {minecraft_version} отсутствует в манифесте Mojang"
        ))
    })?;

    meta.fetch_json(&entry.url).await
}

pub fn profile(
    paths: &LauncherPaths,
    instance: &Instance,
    package: &VersionPackage,
    ctx: &RuntimeContext,
) -> CommandResult<ResolvedProfile> {
    let main_class = package.main_class.clone().ok_or_else(|| {
        CommandError::manifest(format!("В манифесте версии {} нет mainClass", package.id))
    })?;

    Ok(ResolvedProfile {
        version_id: package.id.clone(),
        version_type: "Vanilla".into(),
        main_class,
        assets_id: assets_id(package),
        asset_index: package.asset_index.clone(),
        client_download: package.downloads.as_ref().and_then(|d| d.client.clone()),
        libraries: resolve_libraries(&package.libraries, ctx),
        main_jar: paths.instance(&instance.id).client_jar(),
        java: JavaRequirement::from_package(package),
        arguments: arguments(package),
    })
}

pub fn assets_id(package: &VersionPackage) -> String {
    package
        .assets
        .clone()
        .or_else(|| package.asset_index.as_ref().map(|index| index.id.clone()))
        .unwrap_or_else(|| "legacy".to_string())
}

pub fn arguments(package: &VersionPackage) -> ResolvedArguments {
    match &package.arguments {
        Some(arguments) => ResolvedArguments {
            game: arguments.game.clone(),
            jvm: arguments.jvm.clone(),
            legacy_game: None,
        },
        None => ResolvedArguments {
            game: Vec::new(),
            jvm: Vec::new(),
            legacy_game: package.minecraft_arguments.clone(),
        },
    }
}
