use std::process::{Command, Stdio};

pub fn get_java_list() -> Vec<String> {
    let output = if cfg!(target_os = "windows") {
        Command::new("where").arg("java").output()
    } else {
        Command::new("which").arg("-a").arg("java").output()
    };

    match output {
        Ok(out) => {
            let paths = String::from_utf8_lossy(&out.stdout);
            paths.lines().map(|s| s.trim().to_string()).collect()
        }
        Err(_) => vec![],
    }
}

pub fn get_java_version(java_path: String) -> Option<String> {
    let output = Command::new(&java_path)
        .arg("-version")
        .stderr(Stdio::piped())
        .output()
        .ok()?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    let version_line = stderr.lines().next()?;
    Some(version_line.to_string())
}