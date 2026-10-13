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

    args.extend(inputs.config.custom_jvm_args());

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
        return ResolvedArguments::LEGACY_JVM
            .iter()
            .map(|arg| arg.to_string())
            .collect();
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

/// Splits arguments typed in the settings. Backslashes are kept as is, so Windows paths need no escaping.
pub fn split_user_args(text: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;

    for symbol in text.chars() {
        match quote {
            Some(open) if symbol == open => quote = None,
            Some(_) => current.push(symbol),
            None if symbol == '"' || symbol == '\'' => quote = Some(symbol),
            None if symbol.is_whitespace() => {
                if !current.is_empty() {
                    args.push(std::mem::take(&mut current));
                }
            }
            None => current.push(symbol),
        }
    }

    if !current.is_empty() {
        args.push(current);
    }

    args
}

/// The reverse of `split_user_args`, for arguments that come as a list.
pub fn join_user_args(args: &[String]) -> String {
    args.iter()
        .filter(|arg| !arg.is_empty())
        .map(|arg| quote_user_arg(arg))
        .collect::<Vec<_>>()
        .join(" ")
}

fn quote_user_arg(arg: &str) -> String {
    let plain =
        !arg.contains(|symbol: char| symbol.is_whitespace() || symbol == '"' || symbol == '\'');

    if plain {
        return arg.to_string();
    }

    let mut out = String::with_capacity(arg.len() + 2);
    out.push('"');

    for symbol in arg.chars() {
        match symbol {
            '"' => out.push_str("\"'\"'\""),
            other => out.push(other),
        }
    }

    out.push('"');
    out
}

pub fn substitute(args: &[String], placeholders: &Placeholders) -> Vec<String> {
    args.iter()
        .map(|arg| substitute_one(arg, placeholders))
        .collect()
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
        (
            "auth_uuid".to_string(),
            inputs.account.uuid.clone().unwrap_or_default(),
        ),
        (
            "auth_access_token".to_string(),
            inputs
                .account
                .access_token
                .clone()
                .unwrap_or_else(|| "null".into()),
        ),
        (
            "auth_xuid".to_string(),
            inputs.account.xbl_hash.clone().unwrap_or_default(),
        ),
        (
            "user_type".to_string(),
            inputs.account.user_type().to_string(),
        ),
        ("clientid".to_string(), uuid::Uuid::new_v4().to_string()),
        (
            "version_name".to_string(),
            inputs.profile.version_id.clone(),
        ),
        (
            "version_type".to_string(),
            inputs.profile.version_type.clone(),
        ),
        (
            "assets_index_name".to_string(),
            inputs.profile.assets_id.clone(),
        ),
        (
            "game_directory".to_string(),
            inputs.instance.minecraft().display().to_string(),
        ),
        (
            "assets_root".to_string(),
            inputs.paths.assets().display().to_string(),
        ),
        (
            "game_assets".to_string(),
            inputs.paths.assets().display().to_string(),
        ),
        (
            "natives_directory".to_string(),
            inputs.natives_dir.display().to_string(),
        ),
        (
            "library_directory".to_string(),
            inputs.paths.libraries().display().to_string(),
        ),
        ("classpath".to_string(), classpath),
        ("classpath_separator".to_string(), separator.to_string()),
        ("launcher_name".to_string(), "cast-launcher".to_string()),
        (
            "launcher_version".to_string(),
            env!("CARGO_PKG_VERSION").to_string(),
        ),
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
            &[
                "--username".into(),
                "${auth_player_name}".into(),
                "-cp".into(),
                "${classpath}".into(),
            ],
            &values,
        );

        assert_eq!(args, vec!["--username", "Steve", "-cp", "a;b"]);
    }

    #[test]
    fn keeps_unknown_placeholders_intact() {
        let values = placeholders(&[("a", "1")]);

        assert_eq!(substitute(&["${a}/${b}".into()], &values), vec!["1/${b}"]);
        assert_eq!(
            substitute(&["${unclosed".into()], &values),
            vec!["${unclosed"]
        );
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
        assert_eq!(
            neoforge.len(),
            1,
            "the loader client stays out of the classpath"
        );
    }

    #[test]
    fn user_arguments_are_split_by_whitespace_and_grouped_by_quotes() {
        assert_eq!(
            split_user_args("  -Xss4M\t-XX:+UseZGC\n-Dfile.encoding=UTF-8  "),
            vec!["-Xss4M", "-XX:+UseZGC", "-Dfile.encoding=UTF-8"]
        );

        assert_eq!(
            split_user_args(r#"-Dname="Моя сборка" '-Dquote="x"' "-Dit's=1""#),
            vec!["-Dname=Моя сборка", "-Dquote=\"x\"", "-Dit's=1"]
        );
    }

    #[test]
    fn backslashes_in_user_arguments_are_kept() {
        assert_eq!(
            split_user_args(r#""-Djava.io.tmpdir=C:\Program Files\tmp\" -Dx=D:\games"#),
            vec![r"-Djava.io.tmpdir=C:\Program Files\tmp\", r"-Dx=D:\games"]
        );
    }

    #[test]
    fn empty_user_arguments_give_nothing() {
        assert!(split_user_args("").is_empty());
        assert!(split_user_args("   \n ").is_empty());
        assert!(
            split_user_args(r#""" ''"#).is_empty(),
            "an empty argument would become the main class"
        );
    }

    #[test]
    fn an_unclosed_quote_takes_the_rest_of_the_line() {
        assert_eq!(
            split_user_args(r#"-Xss4M "-Dname=a b"#),
            vec!["-Xss4M", "-Dname=a b"]
        );
    }

    #[test]
    fn joined_user_arguments_split_back_into_the_same_list() {
        let args: Vec<String> = [
            "-XX:+UseG1GC",
            r"-Dpath=C:\Program Files\Java",
            "-Dquote=\"x\"",
            "-Dname=it's mine",
            "-Dmixed=\"it's\" here",
        ]
        .map(String::from)
        .to_vec();

        let joined = join_user_args(&args);

        assert!(joined.starts_with("-XX:+UseG1GC "), "{joined}");
        assert_eq!(split_user_args(&joined), args);
    }

    #[test]
    fn arguments_from_the_settings_go_right_before_the_main_class() {
        use crate::mojang::profile::{GameJar, JavaRequirement};

        let paths = LauncherPaths::new(PathBuf::from("/cfg"), None);
        let instance = paths.instance("abc");
        let account: Account =
            serde_json::from_value(serde_json::json!({ "type": "offline", "name": "Steve" }))
                .unwrap();

        let profile = ResolvedProfile {
            version_id: "1.21.1".into(),
            version_type: "release".into(),
            main_class: "net.minecraft.client.main.Main".into(),
            assets_id: "17".into(),
            asset_index: None,
            client_download: None,
            libraries: Vec::new(),
            main_jar: GameJar::classpath(PathBuf::from("/cfg/versions/1.21.1.jar")),
            java: JavaRequirement::default(),
            arguments: ResolvedArguments {
                jvm: serde_json::from_value(serde_json::json!([
                    "-Dversion.own=1",
                    "-cp",
                    "${classpath}"
                ]))
                .unwrap(),
                ..Default::default()
            },
        };

        let mut config = AppConfig::defaults(Path::new("/cfg"));
        config.java.jvm_args = r#"-XX:+UseZGC "-Dpath=C:\Program Files\x""#.into();

        let command = build(&LaunchInputs {
            paths: &paths,
            instance: &instance,
            profile: &profile,
            config: &config,
            account: &account,
            java_path: "java",
            ctx: &ctx(),
            natives_dir: Path::new("/cfg/natives"),
        });

        let main = command
            .args
            .iter()
            .position(|arg| arg == "net.minecraft.client.main.Main")
            .unwrap();
        let own = command
            .args
            .iter()
            .position(|arg| arg == "-Dversion.own=1")
            .unwrap();

        assert_eq!(
            command.args[main - 2..main],
            ["-XX:+UseZGC", r"-Dpath=C:\Program Files\x"]
        );
        assert!(own < main - 2, "the version's arguments come first");
        assert_eq!(command.args[..2], ["-Xms1024M", "-Xmx4096M"]);
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
        assert_eq!(
            game,
            vec![
                "--username",
                "${auth_player_name}",
                "--version",
                "${version_name}"
            ]
        );
    }
}
