use serde::Serialize;

pub const METADATA: &str =
    "https://maven.neoforged.net/releases/net/neoforged/neoforge/maven-metadata.xml";

pub const LEGACY_MINECRAFT: &str = "1.20.1";

pub const LEGACY_METADATA: &str =
    "https://maven.neoforged.net/releases/net/neoforged/forge/maven-metadata.xml";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Release {
    pub version: String,
    pub minecraft_version: String,
}

pub fn is_legacy(version: &str) -> bool {
    version.starts_with("1.")
}

pub fn minecraft_version(version: &str) -> Option<String> {
    if is_legacy(version) {
        return version.split('-').next().map(str::to_string);
    }

    if version.contains('+') {
        return None;
    }

    let core = version.split('-').next()?;
    let parts: Vec<&str> = core.split('.').collect();

    if !parts.iter().all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit())) {
        return None;
    }

    let mut minecraft = match parts.len() {
        3 => vec!["1", parts[0], parts[1]],
        4 => vec![parts[0], parts[1], parts[2]],
        _ => return None,
    };

    if minecraft.last() == Some(&"0") {
        minecraft.pop();
    }

    Some(minecraft.join("."))
}

pub fn maven_version(minecraft: &str, version: &str) -> String {
    if is_legacy(version) || minecraft != LEGACY_MINECRAFT {
        return version.to_string();
    }

    format!("{LEGACY_MINECRAFT}-{version}")
}

pub fn releases(metadata: &str, legacy_metadata: &str) -> Vec<Release> {
    let of = |xml: &str| {
        super::forge::parse_maven_versions(xml)
            .into_iter()
            .filter_map(|version| {
                minecraft_version(&version).map(|minecraft_version| Release {
                    version,
                    minecraft_version,
                })
            })
            .collect::<Vec<_>>()
    };

    let mut releases = of(metadata);
    releases.extend(of(legacy_metadata));
    releases
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_version_names_the_minecraft_release_it_targets() {
        assert_eq!(minecraft_version("21.1.243").as_deref(), Some("1.21.1"));
        assert_eq!(minecraft_version("20.2.12-beta").as_deref(), Some("1.20.2"));
        assert_eq!(minecraft_version("20.4.190").as_deref(), Some("1.20.4"));
        assert_eq!(minecraft_version("26.1.2.86").as_deref(), Some("26.1.2"));
        assert_eq!(minecraft_version("1.20.1-47.1.106").as_deref(), Some("1.20.1"));
    }

    #[test]
    fn a_zero_patch_is_dropped_the_way_mojang_drops_it() {
        assert_eq!(minecraft_version("21.0.167").as_deref(), Some("1.21"));
        assert_eq!(minecraft_version("26.2.0.37-beta").as_deref(), Some("26.2"));
    }

    #[test]
    fn builds_for_snapshots_have_no_release_to_map_to() {
        assert!(minecraft_version("26.1.0.0-alpha.1+snapshot-1").is_none());
        assert!(minecraft_version("0.25w14craftmine.5-beta").is_none());
        assert!(minecraft_version("").is_none());
        assert!(minecraft_version("21.1").is_none());
    }

    #[test]
    fn only_the_1_20_1_branch_gets_the_game_version_in_its_maven_name() {
        assert_eq!(maven_version("1.20.1", "47.1.106"), "1.20.1-47.1.106");
        assert_eq!(maven_version("1.20.1", "1.20.1-47.1.106"), "1.20.1-47.1.106");
        assert_eq!(maven_version("1.21.1", "21.1.243"), "21.1.243");
        assert_eq!(maven_version("26.1.2", "26.1.2.86"), "26.1.2.86");
    }

    #[test]
    fn both_branches_come_back_as_one_list() {
        let metadata = r#"
            <metadata><versioning><versions>
                <version>20.2.12-beta</version>
                <version>26.1.0.0-alpha.1+snapshot-1</version>
                <version>21.1.243</version>
            </versions></versioning></metadata>
        "#;

        let legacy = r#"
            <metadata><versioning><versions>
                <version>1.20.1-47.1.105</version>
                <version>1.20.1-47.1.106</version>
            </versions></versioning></metadata>
        "#;

        let releases = releases(metadata, legacy);

        assert_eq!(
            releases,
            vec![
                Release { version: "21.1.243".into(), minecraft_version: "1.21.1".into() },
                Release { version: "20.2.12-beta".into(), minecraft_version: "1.20.2".into() },
                Release { version: "1.20.1-47.1.106".into(), minecraft_version: "1.20.1".into() },
                Release { version: "1.20.1-47.1.105".into(), minecraft_version: "1.20.1".into() },
            ],
            "сборка под снапшот отброшена, внутри ветки - свежие сверху"
        );
    }

    #[test]
    fn the_1_20_1_branch_is_the_only_one_named_after_the_game() {
        assert!(is_legacy("1.20.1-47.1.106"));
        assert!(!is_legacy("21.1.243"));
        assert!(!is_legacy("26.1.2.86"));
    }
}
