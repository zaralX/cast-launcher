use std::process::{Command, Stdio};

pub fn get_java_list() -> Vec<String> {
    let command = if cfg!(target_os = "windows") {
        Command::new("where").arg("javaw").output()
    } else {
        Command::new("which").arg("-a").arg("javaw").output()
    };

    command.ok()
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .map(|paths| paths.lines().map(str::trim).map(String::from).collect())
        .unwrap_or_default()
}

pub fn get_java_version(java_path: &str) -> Option<u8> {
    let output = Command::new(java_path)
        .arg("-version")
        .stderr(Stdio::piped())
        .output()
        .ok()?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    let version_line = stderr.lines().find(|line| line.contains("version"))?;

    let version_str = version_line.split_whitespace()
        .find(|s| s.starts_with('"'))?
        .trim_matches('"');

    let major_version = if version_str.starts_with("1.") {
        version_str.split('.').nth(1)? // Для Java 8 и ниже (например, "1.8.0_281")
    } else {
        version_str.split('.').next()? // Для Java 9+ (например, "17.0.1")
    };

    major_version.parse().ok()
}