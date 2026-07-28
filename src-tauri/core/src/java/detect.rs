use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    process::Command,
    thread,
};

use crate::error::{CommandError, CommandResult};
use crate::mojang::rules::{normalize_arch, RuntimeContext};

const PROBE_THREADS: usize = 8;

#[derive(Debug, Clone, serde::Serialize)]
pub struct JavaRuntime {
    pub path: String,
    pub version: String,
    pub major: u32,
    pub vendor: String,
    pub arch: String,
    pub os_version: String,
    pub is_64bit: bool,
    pub source: &'static str,
}

impl JavaRuntime {
    pub fn runtime_context(&self) -> RuntimeContext {
        RuntimeContext::new(&self.arch, &self.os_version)
    }
}

#[derive(Debug, Clone)]
struct Candidate {
    probe: PathBuf,
    launch: PathBuf,
    source: &'static str,
}

type Found = HashMap<PathBuf, Candidate>;

pub async fn list(extra_dirs: Vec<PathBuf>) -> CommandResult<Vec<JavaRuntime>> {
    tokio::task::spawn_blocking(move || {
        let mut runtimes = probe_all(collect_candidates(&extra_dirs));

        runtimes.sort_by(|a, b| {
            b.major
                .cmp(&a.major)
                .then_with(|| b.version.cmp(&a.version))
                .then_with(|| a.path.cmp(&b.path))
        });

        runtimes
    })
    .await
    .map_err(|e| CommandError::task_panicked("поиск Java", e))
}

pub async fn probe(path: String) -> CommandResult<Option<JavaRuntime>> {
    tokio::task::spawn_blocking(move || {
        let raw = Path::new(path.trim());
        if raw.as_os_str().is_empty() {
            return None;
        }

        let mut found: Found = HashMap::new();
        if raw.is_dir() {
            add_home(&mut found, raw, "manual");
        } else if raw.parent().is_some_and(|parent| parent.as_os_str().is_empty()) {
            if let Some(path_var) = std::env::var_os("PATH") {
                for dir in std::env::split_paths(&path_var) {
                    add_exe(&mut found, dir.join(raw), "manual");
                }
            }
        } else {
            add_exe(&mut found, raw.to_path_buf(), "manual");
        }

        found.into_values().find_map(probe_candidate)
    })
    .await
    .map_err(|e| CommandError::task_panicked("проверка Java", e))
}

fn collect_candidates(extra_dirs: &[PathBuf]) -> Vec<Candidate> {
    let mut found: Found = HashMap::new();

    for dir in extra_dirs {
        add_home(&mut found, dir, "launcher");
        scan_dir(&mut found, dir, 3, "launcher");
    }

    scan_path_env(&mut found);

    if let Some(java_home) = std::env::var_os("JAVA_HOME") {
        add_home(&mut found, Path::new(&java_home), "java_home");
    }

    #[cfg(windows)]
    {
        scan_windows_registry(&mut found);
        scan_windows_dirs(&mut found);
    }

    #[cfg(target_os = "linux")]
    scan_linux_dirs(&mut found);

    #[cfg(target_os = "macos")]
    scan_macos_dirs(&mut found);

    scan_minecraft_runtimes(&mut found);

    found.into_values().collect()
}

fn scan_path_env(found: &mut Found) {
    let Some(path_var) = std::env::var_os("PATH") else {
        return;
    };

    for dir in std::env::split_paths(&path_var) {
        add_exe(found, dir.join(java_exe()), "path");
    }
}

fn scan_minecraft_runtimes(found: &mut Found) {
    for root in minecraft_runtime_roots() {
        scan_dir(found, &root, 4, "minecraft");
    }
}

#[cfg(windows)]
fn minecraft_runtime_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();

    if let Some(appdata) = std::env::var_os("APPDATA") {
        roots.push(Path::new(&appdata).join(".minecraft").join("runtime"));
    }

    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        let local = Path::new(&local);
        roots.push(
            local
                .join("Packages")
                .join("Microsoft.4297127D64EC6_8wekyb3d8bbwe")
                .join("LocalCache")
                .join("Local")
                .join("runtime"),
        );
    }

    for base in program_files_dirs() {
        roots.push(base.join("Minecraft Launcher").join("runtime"));
    }

    roots
}

#[cfg(target_os = "linux")]
fn minecraft_runtime_roots() -> Vec<PathBuf> {
    home_dir()
        .map(|home| vec![home.join(".minecraft").join("runtime")])
        .unwrap_or_default()
}

#[cfg(target_os = "macos")]
fn minecraft_runtime_roots() -> Vec<PathBuf> {
    home_dir()
        .map(|home| {
            vec![home
                .join("Library")
                .join("Application Support")
                .join("minecraft")
                .join("runtime")]
        })
        .unwrap_or_default()
}

#[cfg(windows)]
fn program_files_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    for var in ["ProgramFiles", "ProgramFiles(x86)", "ProgramW6432"] {
        if let Some(value) = std::env::var_os(var) {
            let path = PathBuf::from(value);
            if !dirs.contains(&path) {
                dirs.push(path);
            }
        }
    }

    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        dirs.push(Path::new(&local).join("Programs"));
    }

    dirs
}

#[cfg(windows)]
fn scan_windows_dirs(found: &mut Found) {
    const VENDOR_DIRS: &[&str] = &[
        "Java",
        "Eclipse Adoptium",
        "Eclipse Foundation",
        "AdoptOpenJDK",
        "Zulu",
        "Azul",
        "Amazon Corretto",
        "BellSoft",
        "Microsoft",
        "SapMachine",
        "Semeru",
        "IBM",
        "RedHat",
        "Graalvm",
        "JetBrains",
        "Android",
    ];

    for base in program_files_dirs() {
        for vendor in VENDOR_DIRS {
            scan_dir(found, &base.join(vendor), 2, "system");
        }
    }

    if let Some(home) = home_dir() {
        for rel in [".jdks", ".gradle/jdks", "scoop/apps"] {
            scan_dir(found, &home.join(rel), 3, "system");
        }
    }
}

#[cfg(windows)]
fn scan_windows_registry(found: &mut Found) {
    use winreg::{enums::*, RegKey};

    const VALUE_NAMES: &[&str] = &["JavaHome", "InstallationPath", "Path"];

    const KEYS: &[&str] = &[
        "SOFTWARE\\JavaSoft\\Java Runtime Environment",
        "SOFTWARE\\JavaSoft\\Java Development Kit",
        "SOFTWARE\\JavaSoft\\JRE",
        "SOFTWARE\\JavaSoft\\JDK",
        "SOFTWARE\\Eclipse Foundation\\JDK",
        "SOFTWARE\\Eclipse Adoptium\\JDK",
        "SOFTWARE\\Eclipse Adoptium\\JRE",
        "SOFTWARE\\AdoptOpenJDK\\JDK",
        "SOFTWARE\\AdoptOpenJDK\\JRE",
        "SOFTWARE\\Azul Systems\\Zulu",
        "SOFTWARE\\BellSoft\\Liberica",
        "SOFTWARE\\Microsoft\\JDK",
        "SOFTWARE\\Amazon Corretto",
        "SOFTWARE\\IBM\\Semeru Runtime",
        "SOFTWARE\\SapMachine\\JDK",
    ];

    for hive in [HKEY_LOCAL_MACHINE, HKEY_CURRENT_USER] {
        let root = RegKey::predef(hive);

        for key in KEYS {
            for path in [key.to_string(), key.replacen("SOFTWARE\\", "SOFTWARE\\WOW6432Node\\", 1)] {
                if let Ok(opened) = root.open_subkey(&path) {
                    read_registry_homes(found, &opened, VALUE_NAMES, 2);
                }
            }
        }
    }
}

#[cfg(windows)]
fn read_registry_homes(
    found: &mut Found,
    key: &winreg::RegKey,
    value_names: &[&str],
    depth: usize,
) {
    for name in value_names {
        if let Ok(home) = key.get_value::<String, _>(name) {
            add_home(found, Path::new(home.trim()), "registry");
        }
    }

    if depth == 0 {
        return;
    }

    for sub in key.enum_keys().flatten() {
        if let Ok(sub_key) = key.open_subkey(&sub) {
            read_registry_homes(found, &sub_key, value_names, depth - 1);
        }
    }
}

#[cfg(target_os = "linux")]
fn scan_linux_dirs(found: &mut Found) {
    const ROOTS: &[&str] = &[
        "/usr/lib/jvm",
        "/usr/lib64/jvm",
        "/usr/java",
        "/usr/local/java",
        "/opt/java",
        "/opt/jdk",
        "/opt/jdks",
    ];

    for root in ROOTS {
        scan_dir(found, Path::new(root), 2, "system");
    }

    if let Some(home) = home_dir() {
        for rel in [".jdks", ".gradle/jdks", ".sdkman/candidates/java"] {
            scan_dir(found, &home.join(rel), 2, "system");
        }
    }
}

#[cfg(target_os = "macos")]
fn scan_macos_dirs(found: &mut Found) {
    const ROOTS: &[&str] = &[
        "/Library/Java/JavaVirtualMachines",
        "/System/Library/Java/JavaVirtualMachines",
        "/opt/homebrew/opt",
        "/usr/local/opt",
    ];

    for root in ROOTS {
        scan_dir(found, Path::new(root), 2, "system");
    }

    if let Some(home) = home_dir() {
        for rel in [
            "Library/Java/JavaVirtualMachines",
            ".jdks",
            ".gradle/jdks",
            ".sdkman/candidates/java",
        ] {
            scan_dir(found, &home.join(rel), 2, "system");
        }
    }

    if let Ok(output) = new_command(Path::new("/usr/libexec/java_home")).arg("-V").output() {
        let text = String::from_utf8_lossy(&output.stderr).into_owned();
        for line in text.lines() {
            if let Some(pos) = line.find('/') {
                add_home(found, Path::new(line[pos..].trim()), "system");
            }
        }
    }
}

fn java_exe() -> &'static str {
    if cfg!(windows) {
        "java.exe"
    } else {
        "java"
    }
}

fn javaw_exe() -> &'static str {
    if cfg!(windows) {
        "javaw.exe"
    } else {
        "java"
    }
}

fn home_dir() -> Option<PathBuf> {
    let var = if cfg!(windows) { "USERPROFILE" } else { "HOME" };
    std::env::var_os(var).map(PathBuf::from)
}

fn add_home(found: &mut Found, home: &Path, source: &'static str) {
    for rel in ["bin", "jre/bin", "Contents/Home/bin"] {
        add_exe(found, home.join(rel).join(java_exe()), source);
    }
}

fn add_exe(found: &mut Found, exe: PathBuf, source: &'static str) {
    let exe = if exe.is_file() {
        exe
    } else if cfg!(windows) && exe.extension().is_none() && exe.with_extension("exe").is_file() {
        exe.with_extension("exe")
    } else {
        return;
    };

    let probe = exe.with_file_name(java_exe());
    let probe = if probe.is_file() { probe } else { exe.clone() };

    let launch = exe.with_file_name(javaw_exe());
    let launch = if launch.is_file() { launch } else { exe };

    let key = std::fs::canonicalize(&probe).unwrap_or_else(|_| probe.clone());

    found.entry(key).or_insert(Candidate {
        probe,
        launch,
        source,
    });
}

fn scan_dir(found: &mut Found, root: &Path, depth: usize, source: &'static str) {
    if depth == 0 {
        return;
    }

    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };

    for entry in entries.flatten().take(256) {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        add_home(found, &path, source);
        scan_dir(found, &path, depth - 1, source);
    }
}

fn probe_all(candidates: Vec<Candidate>) -> Vec<JavaRuntime> {
    if candidates.is_empty() {
        return Vec::new();
    }

    let threads = PROBE_THREADS.min(candidates.len());
    let mut groups: Vec<Vec<Candidate>> = vec![Vec::new(); threads];
    for (i, candidate) in candidates.into_iter().enumerate() {
        groups[i % threads].push(candidate);
    }

    thread::scope(|scope| {
        let handles: Vec<_> = groups
            .into_iter()
            .map(|group| scope.spawn(move || group.into_iter().filter_map(probe_candidate).collect::<Vec<_>>()))
            .collect();

        handles
            .into_iter()
            .filter_map(|handle| handle.join().ok())
            .flatten()
            .collect()
    })
}

fn probe_candidate(candidate: Candidate) -> Option<JavaRuntime> {
    let output = new_command(&candidate.probe)
        .arg("-XshowSettings:properties")
        .arg("-version")
        .output()
        .ok()?;

    let text = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout)
    );

    let props = parse_properties(&text);

    let version = props
        .get("java.version")
        .cloned()
        .or_else(|| parse_banner_version(&text))?;

    let arch = props
        .get("os.arch")
        .map(|arch| normalize_arch(arch))
        .unwrap_or_else(|| "unknown".to_string());

    let vendor = props
        .get("java.vendor")
        .cloned()
        .or_else(|| props.get("java.runtime.name").cloned())
        .unwrap_or_else(|| "Unknown".to_string());

    let os_version = props.get("os.version").cloned().unwrap_or_default();

    let is_64bit = matches!(
        arch.as_str(),
        "x86_64" | "arm64" | "ppc64" | "ppc64le" | "s390x" | "riscv64" | "sparcv9"
    ) || text.contains("64-Bit");

    Some(JavaRuntime {
        path: candidate.launch.to_string_lossy().to_string(),
        major: parse_major(&version),
        version,
        vendor,
        arch,
        os_version,
        is_64bit,
        source: candidate.source,
    })
}

fn new_command(program: &Path) -> Command {
    let mut command = Command::new(program);

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    command
}

fn parse_properties(text: &str) -> HashMap<String, String> {
    text.lines()
        .filter_map(|line| {
            let (key, value) = line.split_once('=')?;
            let key = key.trim();

            if key.is_empty() || key.contains(char::is_whitespace) {
                return None;
            }

            Some((key.to_string(), value.trim().to_string()))
        })
        .collect()
}

fn parse_banner_version(text: &str) -> Option<String> {
    let start = text.find("version \"")? + "version \"".len();
    let rest = &text[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

fn parse_major(version: &str) -> u32 {
    let mut parts = version
        .split(|c: char| !c.is_ascii_digit())
        .filter(|part| !part.is_empty())
        .filter_map(|part| part.parse::<u32>().ok());

    match parts.next() {
        Some(1) => parts.next().unwrap_or(1),
        Some(major) => major,
        None => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn major_from_legacy_and_modern_versions() {
        assert_eq!(parse_major("1.8.0_432"), 8);
        assert_eq!(parse_major("1.7.0"), 7);
        assert_eq!(parse_major("21.0.5"), 21);
        assert_eq!(parse_major("17"), 17);
        assert_eq!(parse_major(""), 0);
    }

    #[test]
    fn properties_ignore_multiline_values() {
        let text = "    java.version = 21.0.5\n        C:\\some\\path.jar\n    os.arch = amd64\n";
        let props = parse_properties(text);

        assert_eq!(props.get("java.version").map(String::as_str), Some("21.0.5"));
        assert_eq!(props.get("os.arch").map(String::as_str), Some("amd64"));
        assert_eq!(props.len(), 2);
    }

    #[test]
    fn banner_version_fallback() {
        let text = "openjdk version \"1.8.0_432\"\nOpenJDK Runtime Environment";
        assert_eq!(parse_banner_version(text).as_deref(), Some("1.8.0_432"));
    }
}
