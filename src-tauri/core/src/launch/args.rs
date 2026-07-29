use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::account::Account;
use crate::config::AppConfig;
use crate::mojang::profile::{ResolvedArguments, ResolvedProfile};
use crate::mojang::rules::{check_rules, Features, RuntimeContext};
use crate::mojang::version::Argument;
use crate::paths::{InstancePaths, LauncherPaths};

pub struct Placeholders(HashMap<String, String>);

impl Placeholders {
    pub fn get(&self, key: &str) -> Option<&String> {
        self.0.get(key)
    }
}

#[derive(Debug, Clone)]
pub struct LaunchCommand {
    pub java_path: String,
    pub args: Vec<String>,
    pub working_dir: PathBuf,
}

pub struct LaunchInputs<'a> {
    pub paths: &'a LauncherPaths,
    pub instance: &'a InstancePaths,
    pub profile: &'a ResolvedProfile,
    pub config: &'a AppConfig,
    pub account: &'a Account,
    pub java_path: &'a str,
    pub ctx: &'a RuntimeContext,
    pub natives_dir: &'a Path,
}

pub fn build(inputs: &LaunchInputs<'_>) -> LaunchCommand {
    let classpath = classpath(inputs.paths, inputs.profile);
    let placeholders = placeholders(inputs, &classpath);
    let features = Features::new();

    let mut args = Vec::new();

    args.extend(inputs.config.heap_args());

    let jvm = jvm_arguments(&inputs.profile.arguments, inputs.ctx, &features);
    args.extend(substitute(&jvm, &placeholders));

    args.push(inputs.profile.main_class.clone());

    let game = game_arguments(&inputs.profile.arguments, inputs.ctx, &features);
    args.extend(substitute(&game, &placeholders));

    LaunchCommand {
        java_path: inputs.java_path.to_string(),
        args,
        working_dir: inputs.instance.minecraft(),
    }
}

pub fn classpath(paths: &LauncherPaths, profile: &ResolvedProfile) -> Vec<PathBuf> {
    let mut entries: Vec<PathBuf> = profile
        .libraries
        .iter()
        .filter_map(|library| library.artifact.as_ref())
        .map(|artifact| paths.library(&artifact.path))
        .collect();

    if profile.main_jar.on_classpath {
        entries.push(profile.main_jar.path.clone());
    }

    entries
}

pub fn native_jars(paths: &LauncherPaths, profile: &ResolvedProfile) -> Vec<PathBuf> {
    profile
        .libraries
        .iter()
        .filter_map(|library| library.native.as_ref())
        .map(|artifact| paths.library(&artifact.path))
        .collect()
}

fn jvm_arguments(
    arguments: &ResolvedArguments,
    ctx: &RuntimeContext,
    features: &Features,
) -> Vec<String> {
    if arguments.jvm.is_empty() {
        return ResolvedArguments::LEGACY_JVM.iter().map(|arg| arg.to_string()).collect();
    }

    filter(&arguments.jvm, ctx, features)
}

fn game_arguments(
    arguments: &ResolvedArguments,
    ctx: &RuntimeContext,
    features: &Features,
) -> Vec<String> {
    if let Some(legacy) = &arguments.legacy_game {
        return legacy.split_whitespace().map(str::to_string).collect();
    }

    filter(&arguments.game, ctx, features)
}

pub fn filter(arguments: &[Argument], ctx: &RuntimeContext, features: &Features) -> Vec<String> {
    let mut result = Vec::new();

    for argument in arguments {
        match argument {
            Argument::Plain(value) => result.push(value.clone()),
            Argument::Conditional { rules, value } => {
                if !check_rules(rules.as_deref(), ctx, features) {
                    continue;
                }

                result.extend(value.iter().cloned());
            }
        }
    }

    result
}

pub fn substitute(args: &[String], placeholders: &Placeholders) -> Vec<String> {
    args.iter().map(|arg| substitute_one(arg, placeholders)).collect()
}

fn substitute_one(arg: &str, placeholders: &Placeholders) -> String {
    let mut out = String::with_capacity(arg.len());
    let mut rest = arg;

    while let Some(start) = rest.find("${") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];

        let Some(end) = after.find('}') else {
            out.push_str(&rest[start..]);
            return out;
        };

        let key = &after[..end];
        match placeholders.get(key) {
            Some(value) => out.push_str(value),
            None => {
                out.push_str("${");
                out.push_str(key);
                out.push('}');
            }
        }

        rest = &after[end + 1..];
    }

    out.push_str(rest);
    out
}

fn placeholders(inputs: &LaunchInputs<'_>, classpath: &[PathBuf]) -> Placeholders {
    let separator = if cfg!(windows) { ";" } else { ":" };

    let classpath = classpath
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(separator);

    let mut values = HashMap::from([
        ("auth_player_name".to_string(), inputs.account.name.clone()),
        ("auth_uuid".to_string(), inputs.account.uuid.clone().unwrap_or_default()),
        (
            "auth_access_token".to_string(),
            inputs.account.access_token.clone().unwrap_or_else(|| "null".into()),
        ),
        ("auth_xuid".to_string(), inputs.account.xbl_hash.clone().unwrap_or_default()),
        ("user_type".to_string(), inputs.account.user_type().to_string()),
        ("clientid".to_string(), uuid::Uuid::new_v4().to_string()),
        ("version_name".to_string(), inputs.profile.version_id.clone()),
        ("version_type".to_string(), inputs.profile.version_type.clone()),
        ("assets_index_name".to_string(), inputs.profile.assets_id.clone()),
        ("game_directory".to_string(), inputs.instance.minecraft().display().to_string()),
        ("assets_root".to_string(), inputs.paths.assets().display().to_string()),
        ("game_assets".to_string(), inputs.paths.assets().display().to_string()),
        ("natives_directory".to_string(), inputs.natives_dir.display().to_string()),
        ("library_directory".to_string(), inputs.paths.libraries().display().to_string()),
        ("classpath".to_string(), classpath),
        ("classpath_separator".to_string(), separator.to_string()),
        ("launcher_name".to_string(), "cast-launcher".to_string()),
        ("launcher_version".to_string(), env!("CARGO_PKG_VERSION").to_string()),
    ]);

    values.insert("auth_session".into(), values["auth_access_token"].clone());
    values.insert("user_properties".into(), "{}".into());

    Placeholders(values)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mojang::rules::MojangOs;

    fn placeholders(pairs: &[(&str, &str)]) -> Placeholders {
        Placeholders(
            pairs
                .iter()
                .map(|(key, value)| (key.to_string(), value.to_string()))
                .collect(),
        )
    }

    fn ctx() -> RuntimeContext {
        RuntimeContext {
            os: MojangOs::Windows,
            arch: "x86_64".into(),
            os_version: "10.0".into(),
        }
    }

    #[test]
    fn substitutes_known_placeholders() {
        let values = placeholders(&[("auth_player_name", "Steve"), ("classpath", "a;b")]);

        let args = substitute(
            &["--username".into(), "${auth_player_name}".into(), "-cp".into(), "${classpath}".into()],
            &values,
        );

        assert_eq!(args, vec!["--username", "Steve", "-cp", "a;b"]);
    }

    #[test]
    fn keeps_unknown_placeholders_intact() {
        let values = placeholders(&[("a", "1")]);

        assert_eq!(substitute(&["${a}/${b}".into()], &values), vec!["1/${b}"]);
        assert_eq!(substitute(&["${unclosed".into()], &values), vec!["${unclosed"]);
    }

    #[test]
    fn substitutes_several_placeholders_in_one_argument() {
        let values = placeholders(&[("a", "x"), ("b", "y")]);
        assert_eq!(substitute(&["-D${a}=${b}".into()], &values), vec!["-Dx=y"]);
    }

    #[test]
    fn dollar_signs_in_values_are_not_reinterpreted() {
        let values = placeholders(&[("token", "ab$cd${e}")]);
        assert_eq!(substitute(&["${token}".into()], &values), vec!["ab$cd${e}"]);
    }

    #[test]
    fn filters_arguments_by_rules() {
        let arguments: Vec<Argument> = serde_json::from_value(serde_json::json!([
            "--always",
            { "rules": [{ "action": "allow", "os": { "name": "windows" } }], "value": "--on-windows" },
            { "rules": [{ "action": "allow", "os": { "name": "osx" } }], "value": ["--on-mac"] },
            { "value": "--no-rules" }
        ]))
        .unwrap();

        let filtered = filter(&arguments, &ctx(), &Features::new());

        assert_eq!(filtered, vec!["--always", "--on-windows", "--no-rules"]);
    }

    #[test]
    fn the_game_jar_joins_the_classpath_only_when_the_loader_does_not_find_it_itself() {
        use crate::mojang::profile::{GameJar, JavaRequirement, ResolvedLibrary};

        let paths = LauncherPaths::new(PathBuf::from("/cfg"), None);

        let profile = |main_jar: GameJar| ResolvedProfile {
            version_id: "1.21.1".into(),
            version_type: "NeoForge".into(),
            main_class: "cpw.mods.bootstraplauncher.BootstrapLauncher".into(),
            assets_id: "17".into(),
            asset_index: None,
            client_download: None,
            libraries: vec![ResolvedLibrary {
                name: Some("org.ow2.asm:asm:9.7".into()),
                artifact: Some(crate::mojang::profile::ResolvedArtifact {
                    path: "org/ow2/asm/asm/9.7/asm-9.7.jar".into(),
                    url: None,
                    sha1: None,
                    size: None,
                }),
                native: None,
            }],
            main_jar,
            java: JavaRequirement::default(),
            arguments: ResolvedArguments::default(),
        };

        let patched = paths.library("net/neoforged/neoforge/21.1.243/neoforge-21.1.243-client.jar");

        let vanilla = classpath(&paths, &profile(GameJar::classpath(patched.clone())));
        assert_eq!(vanilla.len(), 2);
        assert_eq!(vanilla[1], patched);

        let neoforge = classpath(&paths, &profile(GameJar::found_by_loader(patched)));
        assert_eq!(neoforge.len(), 1, "клиент загрузчика в classpath не идёт");
    }

    #[test]
    fn legacy_versions_get_default_jvm_arguments() {
        let arguments = ResolvedArguments {
            game: Vec::new(),
            jvm: Vec::new(),
            legacy_game: Some("--username ${auth_player_name} --version ${version_name}".into()),
        };

        let jvm = jvm_arguments(&arguments, &ctx(), &Features::new());
        assert_eq!(jvm, ResolvedArguments::LEGACY_JVM.to_vec());

        let game = game_arguments(&arguments, &ctx(), &Features::new());
        assert_eq!(game, vec!["--username", "${auth_player_name}", "--version", "${version_name}"]);
    }
}
