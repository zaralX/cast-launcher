use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use sha1::{Digest, Sha1};
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, BufReader};
use tokio::process::Command;

use crate::error::{CommandError, CommandResult};
use crate::fs_util::{ensure_dir, remove_file_if_exists, safe_join};
use crate::mojang::maven::Gradle;

use super::installer::Processor;

const TAIL_LINES: usize = 40;
const STEP_TIMEOUT: Duration = Duration::from_secs(900);
const POLL_INTERVAL: Duration = Duration::from_millis(150);

pub struct ProcessorEnv<'a> {
    pub java: &'a str,
    pub libraries: &'a Path,
    pub installer: &'a Path,
    pub minecraft_jar: &'a Path,
    pub minecraft_version: &'a str,
    pub root: &'a Path,
    pub scratch: &'a Path,
}

pub async fn run<S, C>(
    processors: &[Processor],
    data: &BTreeMap<String, String>,
    env: &ProcessorEnv<'_>,
    on_step: S,
    cancelled: C,
) -> CommandResult<()>
where
    S: Fn(usize, usize, &str),
    C: Fn() -> bool,
{
    if processors.is_empty() {
        return Ok(());
    }

    if !env.minecraft_jar.is_file() {
        return Err(CommandError::forge(
            "Для сборки Forge нужен клиент Minecraft, но его файл отсутствует",
        )
        .with_details(env.minecraft_jar.display().to_string()));
    }

    ensure_dir(env.root).await?;
    ensure_dir(env.scratch).await?;

    let tokens = tokens(data, env).await?;
    let total = processors.len();

    for (index, processor) in processors.iter().enumerate() {
        if cancelled() {
            return Err(CommandError::aborted("Установка прервана"));
        }

        let name = short_name(&processor.jar);
        on_step(index, total, &name);

        let outputs = outputs(processor, &tokens, env.libraries)?;

        if !outputs.is_empty() && verify(&outputs).await.is_ok() {
            continue;
        }

        for (path, _) in &outputs {
            remove_file_if_exists(path).await;
        }

        let classpath = classpath(processor, env.libraries)?;
        let main_class = main_class(&classpath[0]).await?;
        let args = arguments(&processor.args, &tokens, env.libraries)?;

        execute(env.java, &classpath, &main_class, &args, &name, &cancelled).await?;

        verify(&outputs).await.map_err(|error| {
            CommandError::forge(format!("Шаг сборки Forge «{name}» дал неверный результат"))
                .with_details(error)
        })?;
    }

    Ok(())
}

async fn tokens(
    data: &BTreeMap<String, String>,
    env: &ProcessorEnv<'_>,
) -> CommandResult<HashMap<String, String>> {
    let mut tokens = HashMap::from([
        ("SIDE".to_string(), "client".to_string()),
        ("MINECRAFT_JAR".to_string(), display(env.minecraft_jar)),
        ("MINECRAFT_VERSION".to_string(), env.minecraft_version.to_string()),
        ("ROOT".to_string(), display(env.root)),
        ("INSTALLER".to_string(), display(env.installer)),
        ("LIBRARY_DIR".to_string(), display(env.libraries)),
    ]);

    for (key, raw) in data {
        let value = match artifact(raw) {
            Some(coordinate) => display(&library_path(env.libraries, coordinate)?),
            None => {
                let value = replace_tokens(&tokens, raw)?;

                match value.strip_prefix('/') {
                    Some(_) => display(&unpack(env.installer, &value, env.scratch).await?),
                    None => value,
                }
            }
        };

        tokens.insert(key.clone(), value);
    }

    Ok(tokens)
}

fn artifact(value: &str) -> Option<&str> {
    value
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
}

fn library_path(libraries: &Path, coordinate: &str) -> CommandResult<PathBuf> {
    safe_join(libraries, &Gradle::parse(coordinate)?.path())
}

async fn unpack(installer: &Path, entry: &str, scratch: &Path) -> CommandResult<PathBuf> {
    let target = safe_join(scratch, entry)?;

    if target.is_file() {
        return Ok(target);
    }

    let bytes = crate::archive::read_entry(installer.to_path_buf(), entry.trim_start_matches('/').to_string()).await?;

    if let Some(parent) = target.parent() {
        ensure_dir(parent).await?;
    }

    tokio::fs::write(&target, &bytes)
        .await
        .map_err(|e| CommandError::io("Не удалось распаковать данные Forge", &target, e))?;

    Ok(target)
}

fn arguments(
    args: &[String],
    tokens: &HashMap<String, String>,
    libraries: &Path,
) -> CommandResult<Vec<String>> {
    args.iter()
        .map(|arg| match artifact(arg) {
            Some(coordinate) => Ok(display(&library_path(libraries, coordinate)?)),
            None => replace_tokens(tokens, arg),
        })
        .collect()
}

fn outputs(
    processor: &Processor,
    tokens: &HashMap<String, String>,
    libraries: &Path,
) -> CommandResult<Vec<(PathBuf, Option<String>)>> {
    let mut outputs = Vec::with_capacity(processor.outputs.len());

    for (key, expected) in &processor.outputs {
        let path = match artifact(key) {
            Some(coordinate) => library_path(libraries, coordinate)?,
            None => PathBuf::from(replace_tokens(tokens, key)?),
        };

        let sha1 = match expected {
            Some(value) => Some(replace_tokens(tokens, value)?),
            None => None,
        };

        outputs.push((path, sha1));
    }

    Ok(outputs)
}

fn classpath(processor: &Processor, libraries: &Path) -> CommandResult<Vec<PathBuf>> {
    let mut entries = Vec::with_capacity(processor.classpath.len() + 1);

    for coordinate in std::iter::once(&processor.jar).chain(processor.classpath.iter()) {
        let path = library_path(libraries, coordinate)?;

        if !path.is_file() {
            return Err(CommandError::forge(format!(
                "Для сборки Forge не хватает библиотеки {coordinate}"
            ))
            .with_details(display(&path)));
        }

        entries.push(path);
    }

    Ok(entries)
}

async fn main_class(jar: &Path) -> CommandResult<String> {
    let bytes = crate::archive::read_entry(jar.to_path_buf(), "META-INF/MANIFEST.MF".to_string()).await?;

    manifest_main_class(&String::from_utf8_lossy(&bytes)).ok_or_else(|| {
        CommandError::forge("В инструменте Forge не указан главный класс").with_details(display(jar))
    })
}

fn manifest_main_class(manifest: &str) -> Option<String> {
    let mut unfolded = String::new();

    for line in manifest.split('\n') {
        let line = line.strip_suffix('\r').unwrap_or(line);

        match line.strip_prefix(' ') {
            Some(rest) if !unfolded.is_empty() => unfolded.push_str(rest),
            _ => {
                if let Some(value) = attribute(&unfolded) {
                    return Some(value);
                }
                unfolded.clear();
                unfolded.push_str(line);
            }
        }
    }

    attribute(&unfolded)
}

fn attribute(line: &str) -> Option<String> {
    let value = line.strip_prefix("Main-Class:")?.trim();

    (!value.is_empty()).then(|| value.to_string())
}

fn replace_tokens(tokens: &HashMap<String, String>, value: &str) -> CommandResult<String> {
    let chars: Vec<char> = value.chars().collect();
    let mut out = String::with_capacity(value.len());
    let mut index = 0;

    while index < chars.len() {
        let current = chars[index];

        if current == '\\' {
            index += 1;
            let escaped = chars.get(index).ok_or_else(|| malformed(value))?;
            out.push(*escaped);
            index += 1;
            continue;
        }

        if current != '{' && current != '\'' {
            out.push(current);
            index += 1;
            continue;
        }

        let closing = if current == '{' { '}' } else { '\'' };
        let mut key = String::new();
        let mut cursor = index + 1;
        let mut closed = false;

        while cursor < chars.len() {
            let inner = chars[cursor];

            if inner == '\\' {
                cursor += 1;
                let escaped = chars.get(cursor).ok_or_else(|| malformed(value))?;
                key.push(*escaped);
                cursor += 1;
                continue;
            }

            if inner == closing {
                closed = true;
                break;
            }

            key.push(inner);
            cursor += 1;
        }

        if !closed {
            return Err(malformed(value));
        }

        if current == '\'' {
            out.push_str(&key);
        } else {
            let replacement = tokens.get(&key).ok_or_else(|| {
                CommandError::forge(format!("Установщик Forge ссылается на неизвестное значение {key}"))
                    .with_details(value.to_string())
            })?;

            out.push_str(replacement);
        }

        index = cursor + 1;
    }

    Ok(out)
}

fn malformed(value: &str) -> CommandError {
    CommandError::forge("Установщик Forge содержит некорректный шаблон").with_details(value.to_string())
}

async fn verify(outputs: &[(PathBuf, Option<String>)]) -> Result<(), String> {
    for (path, expected) in outputs {
        if !path.is_file() {
            return Err(format!("нет файла {}", path.display()));
        }

        let Some(expected) = expected else { continue };
        let Some(actual) = file_sha1(path).await else {
            return Err(format!("не удалось прочитать {}", path.display()));
        };

        if &actual != expected {
            return Err(format!(
                "{}\nОжидалось: {expected}\nПолучено:  {actual}",
                path.display()
            ));
        }
    }

    Ok(())
}

async fn file_sha1(path: &Path) -> Option<String> {
    let mut file = tokio::fs::File::open(path).await.ok()?;
    let mut hasher = Sha1::new();
    let mut buffer = vec![0_u8; 128 * 1024];

    loop {
        let read = file.read(&mut buffer).await.ok()?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    let digest = hasher.finalize();
    let mut out = String::with_capacity(digest.len() * 2);

    for byte in digest {
        out.push_str(&format!("{byte:02x}"));
    }

    Some(out)
}

async fn execute<C>(
    java: &str,
    classpath: &[PathBuf],
    main_class: &str,
    args: &[String],
    name: &str,
    cancelled: &C,
) -> CommandResult<()>
where
    C: Fn() -> bool,
{
    let separator = if cfg!(windows) { ";" } else { ":" };
    let classpath = classpath.iter().map(|path| display(path)).collect::<Vec<_>>().join(separator);

    let mut child = new_command(java)
        .arg("-cp")
        .arg(classpath)
        .arg(main_class)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| CommandError::spawn(java, e))?;

    let tail = Arc::new(Mutex::new(Vec::<String>::new()));
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    let logs = tokio::spawn({
        let tail = Arc::clone(&tail);
        async move {
            tokio::join!(drain(stdout, Arc::clone(&tail)), drain(stderr, tail));
        }
    });

    let deadline = Instant::now() + STEP_TIMEOUT;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {}
            Err(error) => {
                let _ = child.start_kill();
                logs.abort();
                return Err(CommandError::forge(format!("Шаг сборки Forge «{name}» прерван"))
                    .with_details(error.to_string()));
            }
        }

        if cancelled() {
            let _ = child.kill().await;
            logs.abort();
            return Err(CommandError::aborted("Установка прервана"));
        }

        if Instant::now() >= deadline {
            let _ = child.kill().await;
            logs.abort();
            return Err(CommandError::forge(format!(
                "Шаг сборки Forge «{name}» не уложился в {} мин",
                STEP_TIMEOUT.as_secs() / 60
            )));
        }

        tokio::time::sleep(POLL_INTERVAL).await;
    };

    let _ = logs.await;

    if status.success() {
        return Ok(());
    }

    let code = status
        .code()
        .map(|code| code.to_string())
        .unwrap_or_else(|| "неизвестен".to_string());

    let details = tail
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .join("\n");

    Err(
        CommandError::forge(format!("Шаг сборки Forge «{name}» завершился с кодом {code}"))
            .with_details(details),
    )
}

async fn drain<R>(reader: Option<R>, tail: Arc<Mutex<Vec<String>>>)
where
    R: AsyncRead + Unpin,
{
    let Some(reader) = reader else { return };
    let mut lines = BufReader::new(reader).lines();

    while let Ok(Some(line)) = lines.next_line().await {
        let mut tail = tail.lock().unwrap_or_else(|poisoned| poisoned.into_inner());

        tail.push(line);

        if tail.len() > TAIL_LINES {
            tail.remove(0);
        }
    }
}

fn short_name(coordinate: &str) -> String {
    Gradle::parse(coordinate)
        .map(|gradle| gradle.artifact)
        .unwrap_or_else(|_| coordinate.to_string())
}

fn display(path: &Path) -> String {
    path.display().to_string()
}

fn new_command(program: &str) -> Command {
    #[allow(unused_mut)]
    let mut command = Command::new(program);

    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    command
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect()
    }

    #[test]
    fn tokens_quotes_and_escapes_follow_the_installer_rules() {
        let values = tokens(&[("ROOT", "C:\\games"), ("SIDE", "client")]);

        assert_eq!(replace_tokens(&values, "{ROOT}/run.sh").unwrap(), "C:\\games/run.sh");
        assert_eq!(replace_tokens(&values, "--side").unwrap(), "--side");
        assert_eq!(replace_tokens(&values, "'20210115.111550'").unwrap(), "20210115.111550");
        assert_eq!(replace_tokens(&values, "--side={SIDE}").unwrap(), "--side=client");
        assert_eq!(replace_tokens(&values, "\\{SIDE\\}").unwrap(), "{SIDE}");
    }

    #[test]
    fn unknown_or_unterminated_tokens_are_errors() {
        let values = tokens(&[("SIDE", "client")]);

        assert!(replace_tokens(&values, "{MISSING}").is_err());
        assert!(replace_tokens(&values, "{SIDE").is_err());
        assert!(replace_tokens(&values, "'unterminated").is_err());
        assert!(replace_tokens(&values, "trailing\\").is_err());
    }

    #[test]
    fn bracketed_values_are_maven_coordinates() {
        assert_eq!(artifact("[g:a:1]"), Some("g:a:1"));
        assert_eq!(artifact("{TOKEN}"), None);

        let path = library_path(
            Path::new("/libs"),
            "net.minecraft:client:1.21.11:mappings@tsrg",
        )
        .unwrap();

        assert!(path.ends_with("net/minecraft/client/1.21.11/client-1.21.11-mappings.tsrg"));
    }

    #[test]
    fn arguments_mix_coordinates_and_tokens() {
        let values = tokens(&[("PATCHED", "/libs/forge-client.jar"), ("SIDE", "client")]);

        let args = arguments(
            &[
                "--clean".to_string(),
                "[net.minecraft:client:1.21.11:official]".to_string(),
                "--output".to_string(),
                "{PATCHED}".to_string(),
                "--side".to_string(),
                "{SIDE}".to_string(),
            ],
            &values,
            Path::new("/libs"),
        )
        .unwrap();

        assert_eq!(args[0], "--clean");
        assert!(args[1].ends_with("client-1.21.11-official.jar"));
        assert_eq!(args[3], "/libs/forge-client.jar");
        assert_eq!(args[5], "client");
    }

    #[test]
    fn outputs_resolve_both_the_path_and_the_checksum() {
        let values = tokens(&[
            ("PATCHED", "/libs/forge-client.jar"),
            ("PATCHED_SHA", "95c071d141e47e75c738d54e760115a68fd483c8"),
        ]);

        let processor = Processor {
            jar: "net.minecraftforge:binarypatcher:1.3.1".into(),
            classpath: Vec::new(),
            args: Vec::new(),
            outputs: BTreeMap::from([
                ("{PATCHED}".to_string(), Some("{PATCHED_SHA}".to_string())),
                ("[g:a:1]".to_string(), None),
            ]),
        };

        let resolved = outputs(&processor, &values, Path::new("/libs")).unwrap();

        assert_eq!(resolved.len(), 2);
        assert!(resolved.iter().any(|(path, sha1)| {
            path == Path::new("/libs/forge-client.jar")
                && sha1.as_deref() == Some("95c071d141e47e75c738d54e760115a68fd483c8")
        }));
        assert!(resolved.iter().any(|(path, sha1)| {
            path.ends_with("g/a/1/a-1.jar") && sha1.is_none()
        }));
    }

    #[test]
    fn main_class_survives_wrapped_manifest_lines() {
        let manifest = "Manifest-Version: 1.0\r\nMain-Class: net.minecraftforge.installert\r\n ools.ConsoleTool\r\nBuild-Jdk: 17\r\n";
        assert_eq!(
            manifest_main_class(manifest).as_deref(),
            Some("net.minecraftforge.installertools.ConsoleTool")
        );

        assert_eq!(
            manifest_main_class("Main-Class: net.md_5.specialsource.SpecialSource\n").as_deref(),
            Some("net.md_5.specialsource.SpecialSource")
        );

        assert!(manifest_main_class("Manifest-Version: 1.0\n").is_none());
    }

    #[test]
    fn processor_names_are_short() {
        assert_eq!(short_name("net.minecraftforge:binarypatcher:1.3.1"), "binarypatcher");
        assert_eq!(short_name("сломано"), "сломано");
    }

    #[tokio::test]
    async fn outputs_are_verified_by_checksum() {
        let dir = std::env::temp_dir().join(format!("cast-forge-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();

        let file = dir.join("out.jar");
        std::fs::write(&file, b"forge").unwrap();

        let sha1 = file_sha1(&file).await.unwrap();

        assert!(verify(&[(file.clone(), Some(sha1))]).await.is_ok());
        assert!(verify(&[(file.clone(), Some("deadbeef".into()))]).await.is_err());
        assert!(verify(&[(file.clone(), None)]).await.is_ok());
        assert!(verify(&[(dir.join("gone.jar"), None)]).await.is_err());

        std::fs::remove_dir_all(&dir).ok();
    }
}
