use std::collections::HashSet;
use std::path::PathBuf;

use crate::error::{CommandError, CommandResult};
use crate::fs_util::read_json;
use crate::instance::Instance;
use crate::mojang::maven::Gradle;
use crate::mojang::profile::{
    resolve_libraries, ResolvedArguments, ResolvedLibrary, ResolvedProfile,
};
use crate::mojang::rules::RuntimeContext;
use crate::mojang::version::VersionPackage;
use crate::paths::LauncherPaths;

pub const FORGE_MAVEN: &str = "https://maven.minecraftforge.net";

pub fn installer_url(forge_version: &str) -> String {
    format!("{FORGE_MAVEN}/net/minecraftforge/forge/{forge_version}/forge-{forge_version}-installer.jar")
}

pub async fn client_package(
    paths: &LauncherPaths,
    forge_version: &str,
) -> CommandResult<VersionPackage> {
    let file = paths.forge_cache(forge_version).client_json();

    if !file.is_file() {
        return Err(CommandError::forge(
            "Forge для этой сборки ещё не установлен. Запустите установку заново.",
        )
        .with_details(file.display().to_string()));
    }

    read_json(&file).await
}

pub fn parse_maven_versions(xml: &str) -> Vec<String> {
    let mut versions: Vec<String> = xml
        .split("<version>")
        .skip(1)
        .filter_map(|chunk| chunk.split_once("</version>"))
        .map(|(version, _)| version.trim().to_string())
        .filter(|version| !version.is_empty())
        .collect();

    versions.reverse();
    versions
}

pub fn profile(
    paths: &LauncherPaths,
    instance: &Instance,
    vanilla: &VersionPackage,
    forge: &VersionPackage,
    ctx: &RuntimeContext,
) -> CommandResult<ResolvedProfile> {
    let mut profile = super::vanilla::profile(paths, instance, vanilla, ctx)?;

    let main_class = forge.main_class.clone().ok_or_else(|| {
        CommandError::forge("В манифесте Forge нет mainClass")
    })?;

    profile.version_type = "Forge".into();
    profile.main_class = main_class;
    profile.libraries = merge_libraries(
        resolve_libraries(&forge.libraries, ctx),
        resolve_libraries(&vanilla.libraries, ctx),
    );
    profile.arguments = merge_arguments(&profile.arguments, forge);

    if brings_own_client(forge) {
        profile.main_jar = patched_client(paths, instance.require_loader_version()?)?;
    }

    Ok(profile)
}

fn brings_own_client(forge: &VersionPackage) -> bool {
    forge.arguments.is_some()
}

fn patched_client(paths: &LauncherPaths, forge_version: &str) -> CommandResult<PathBuf> {
    let coordinate = format!("net.minecraftforge:forge:{forge_version}:client");

    Ok(paths.library(&Gradle::parse(&coordinate)?.path()))
}

fn merge_libraries(
    forge: Vec<ResolvedLibrary>,
    vanilla: Vec<ResolvedLibrary>,
) -> Vec<ResolvedLibrary> {
    let mut seen = HashSet::new();
    let mut merged = Vec::with_capacity(forge.len() + vanilla.len());

    for library in forge.into_iter().chain(vanilla) {
        if let Some(key) = library_key(&library) {
            if !seen.insert(key) {
                continue;
            }
        }

        merged.push(library);
    }

    merged
}

fn library_key(library: &ResolvedLibrary) -> Option<String> {
    let gradle = Gradle::parse(library.name.as_deref()?).ok()?;

    Some(format!(
        "{}:{}:{}",
        gradle.group,
        gradle.artifact,
        gradle.classifier.unwrap_or_default()
    ))
}

fn merge_arguments(vanilla: &ResolvedArguments, forge: &VersionPackage) -> ResolvedArguments {
    match &forge.arguments {
        Some(forge_arguments) => {
            let mut game = vanilla.game.clone();
            game.extend(forge_arguments.game.clone());

            let mut jvm = vanilla.jvm.clone();
            jvm.extend(forge_arguments.jvm.clone());

            ResolvedArguments {
                game,
                jvm,
                legacy_game: None,
            }
        }
        None => ResolvedArguments {
            game: vanilla.game.clone(),
            jvm: vanilla.jvm.clone(),
            legacy_game: forge
                .minecraft_arguments
                .clone()
                .or_else(|| vanilla.legacy_game.clone()),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn package(value: serde_json::Value) -> VersionPackage {
        serde_json::from_value(value).unwrap()
    }

    fn windows() -> RuntimeContext {
        RuntimeContext {
            os: crate::mojang::rules::MojangOs::Windows,
            arch: "x86_64".into(),
            os_version: "10.0".into(),
        }
    }

    fn forge_instance(loader_version: &str) -> Instance {
        serde_json::from_value(json!({
            "id": "abc",
            "name": "Сборка",
            "minecraftVersion": "1.20.1",
            "type": "forge",
            "loaderVersion": loader_version
        }))
        .unwrap()
    }

    fn vanilla_package() -> VersionPackage {
        package(json!({
            "id": "1.20.1",
            "mainClass": "net.minecraft.client.main.Main",
            "libraries": []
        }))
    }

    #[test]
    fn modern_forge_launches_from_the_patched_jar_instead_of_the_vanilla_one() {
        let paths = LauncherPaths::new(std::path::PathBuf::from("/cfg"), None);
        let instance = forge_instance("1.20.1-47.4.13");

        let forge = package(json!({
            "id": "1.20.1-forge-47.4.13",
            "mainClass": "cpw.mods.bootstraplauncher.BootstrapLauncher",
            "arguments": { "game": ["--launchTarget", "forge_client"], "jvm": [] },
            "libraries": []
        }));

        let profile = profile(&paths, &instance, &vanilla_package(), &forge, &windows()).unwrap();

        assert_eq!(
            profile.main_jar,
            paths.library("net/minecraftforge/forge/1.20.1-47.4.13/forge-1.20.1-47.4.13-client.jar")
        );
        assert_ne!(
            profile.main_jar,
            paths.instance("abc").client_jar(),
            "ванильный клиент вторым модулем с теми же пакетами роняет игру"
        );
    }

    #[test]
    fn legacy_forge_keeps_patching_the_vanilla_jar_at_runtime() {
        let paths = LauncherPaths::new(std::path::PathBuf::from("/cfg"), None);
        let instance = forge_instance("1.7.10-10.13.4.1614-1.7.10");

        let forge = package(json!({
            "id": "1.7.10-Forge10.13.4.1614",
            "mainClass": "net.minecraft.launchwrapper.Launch",
            "minecraftArguments": "--username ${auth_player_name} --tweakClass fml",
            "libraries": []
        }));

        let profile = profile(&paths, &instance, &vanilla_package(), &forge, &windows()).unwrap();

        assert_eq!(profile.main_jar, paths.instance("abc").client_jar());
    }

    #[test]
    fn the_launcher_only_swaps_the_jar_and_leaves_the_rest_of_the_profile_alone() {
        let paths = LauncherPaths::new(std::path::PathBuf::from("/cfg"), None);

        let forge = package(json!({
            "id": "1.20.1-forge-47.4.13",
            "mainClass": "cpw.mods.bootstraplauncher.BootstrapLauncher",
            "arguments": { "game": [], "jvm": [] },
            "libraries": []
        }));

        let profile = profile(
            &paths,
            &forge_instance("1.20.1-47.4.13"),
            &vanilla_package(),
            &forge,
            &windows(),
        )
        .unwrap();

        assert_eq!(profile.version_id, "1.20.1", "ассеты и версия остаются ванильными");
        assert_eq!(profile.version_type, "Forge");
        assert_eq!(profile.main_class, "cpw.mods.bootstraplauncher.BootstrapLauncher");
    }

    #[test]
    fn a_forge_instance_without_a_version_cannot_name_its_patched_jar() {
        let paths = LauncherPaths::new(std::path::PathBuf::from("/cfg"), None);

        let instance: Instance = serde_json::from_value(json!({
            "id": "abc",
            "name": "Без версии",
            "minecraftVersion": "1.20.1",
            "type": "forge"
        }))
        .unwrap();

        let forge = package(json!({
            "id": "1.20.1-forge-47.4.13",
            "mainClass": "cpw.mods.bootstraplauncher.BootstrapLauncher",
            "arguments": { "game": [], "jvm": [] },
            "libraries": []
        }));

        let error = profile(&paths, &instance, &vanilla_package(), &forge, &windows()).unwrap_err();

        assert!(error.message.contains("Без версии"));
    }

    #[test]
    fn installer_url_follows_forge_maven_layout() {
        assert_eq!(
            installer_url("1.20.1-47.2.0"),
            "https://maven.minecraftforge.net/net/minecraftforge/forge/1.20.1-47.2.0/forge-1.20.1-47.2.0-installer.jar"
        );
    }

    #[test]
    fn maven_metadata_versions_come_back_newest_first() {
        let xml = r#"
            <metadata><versioning><versions>
                <version>1.12.2-14.23.5.2859</version>
                <version>1.20.1-47.2.0</version>
            </versions></versioning></metadata>
        "#;

        assert_eq!(
            parse_maven_versions(xml),
            vec!["1.20.1-47.2.0", "1.12.2-14.23.5.2859"]
        );
    }

    #[test]
    fn malformed_metadata_yields_nothing() {
        assert!(parse_maven_versions("<metadata/>").is_empty());
        assert!(parse_maven_versions("<version>unterminated").is_empty());
    }

    #[test]
    fn modern_forge_arguments_append_to_vanilla() {
        let vanilla = ResolvedArguments {
            game: serde_json::from_value(json!(["--username", "${auth_player_name}"])).unwrap(),
            jvm: serde_json::from_value(json!(["-cp", "${classpath}"])).unwrap(),
            legacy_game: None,
        };

        let forge = package(json!({
            "id": "forge",
            "arguments": { "game": ["--launchTarget", "forgeclient"], "jvm": ["-DignoreList=x"] },
            "libraries": []
        }));

        let merged = merge_arguments(&vanilla, &forge);

        assert_eq!(merged.game.len(), 4);
        assert_eq!(merged.jvm.len(), 3);
        assert!(merged.legacy_game.is_none());
    }

    #[test]
    fn forge_wins_over_vanilla_for_the_same_artifact() {
        let ctx = RuntimeContext {
            os: crate::mojang::rules::MojangOs::Windows,
            arch: "x86_64".into(),
            os_version: "10.0".into(),
        };

        let forge = package(json!({
            "id": "forge",
            "libraries": [
                { "name": "net.minecraftforge:forge:1.21.11-61.1.14:client", "downloads": { "artifact": { "path": "forge-client.jar", "url": "" } } },
                { "name": "org.ow2.asm:asm:9.7", "downloads": { "artifact": { "path": "asm-9.7.jar", "url": "u" } } }
            ]
        }));

        let vanilla = package(json!({
            "id": "1.21.11",
            "libraries": [
                { "name": "org.ow2.asm:asm:9.5", "downloads": { "artifact": { "path": "asm-9.5.jar", "url": "u" } } },
                { "name": "com.mojang:logging:1.5.10", "downloads": { "artifact": { "path": "logging.jar", "url": "u" } } },
                { "name": "org.lwjgl:lwjgl:3.3.3:natives-windows", "downloads": { "artifact": { "path": "lwjgl-natives.jar", "url": "u" } } }
            ]
        }));

        let merged = merge_libraries(
            resolve_libraries(&forge.libraries, &ctx),
            resolve_libraries(&vanilla.libraries, &ctx),
        );

        let paths: Vec<_> = merged
            .iter()
            .filter_map(|library| library.artifact.as_ref())
            .map(|artifact| artifact.path.as_str())
            .collect();

        assert_eq!(
            paths,
            vec!["forge-client.jar", "asm-9.7.jar", "logging.jar", "lwjgl-natives.jar"]
        );
    }

    #[test]
    fn libraries_without_a_readable_name_are_never_dropped() {
        let unnamed = ResolvedLibrary {
            name: None,
            artifact: None,
            native: None,
        };

        let broken = ResolvedLibrary {
            name: Some("не координата".into()),
            artifact: None,
            native: None,
        };

        let merged = merge_libraries(
            vec![unnamed.clone(), broken.clone()],
            vec![unnamed, broken],
        );

        assert_eq!(merged.len(), 4);
    }

    #[test]
    fn legacy_forge_arguments_replace_vanilla_string() {
        let vanilla = ResolvedArguments {
            game: Vec::new(),
            jvm: Vec::new(),
            legacy_game: Some("--username ${auth_player_name}".into()),
        };

        let forge = package(json!({
            "id": "forge",
            "minecraftArguments": "--username ${auth_player_name} --tweakClass fml",
            "libraries": []
        }));

        let merged = merge_arguments(&vanilla, &forge);

        assert_eq!(
            merged.legacy_game.as_deref(),
            Some("--username ${auth_player_name} --tweakClass fml")
        );
    }
}
