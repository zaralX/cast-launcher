use crate::error::{CommandError, CommandResult};
use crate::fs_util::read_json;
use crate::instance::Instance;
use crate::mojang::profile::{resolve_libraries, ResolvedArguments, ResolvedProfile};
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
    let forge_version = instance.loader_version.as_deref().ok_or_else(|| {
        CommandError::forge("У сборки не указана версия Forge")
    })?;

    let mut profile = super::vanilla::profile(paths, instance, vanilla, ctx)?;

    let main_class = forge.main_class.clone().ok_or_else(|| {
        CommandError::forge("В манифесте Forge нет mainClass")
    })?;

    let mut merged = resolve_libraries(&forge.libraries, ctx);
    merged.extend(resolve_libraries(&vanilla.libraries, ctx));

    profile.version_type = "Forge".into();
    profile.main_class = main_class;
    profile.libraries = merged;
    profile.arguments = merge_arguments(&profile.arguments, forge);
    profile.main_jar = paths.forge_cache(forge_version).client_jar();

    Ok(profile)
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
