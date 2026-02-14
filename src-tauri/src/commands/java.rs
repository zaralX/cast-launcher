use tauri::AppHandle;
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Debug, serde::Serialize)]
pub struct JavaRuntime {
    pub path: String,
}

#[tauri::command]
pub fn list_java(_app: AppHandle) -> Vec<JavaRuntime> {
    let mut found = HashSet::<PathBuf>::new();

    // --- PATH ---
    if let Ok(path) = which_java() {
        found.insert(path);
    }

    // --- JAVA_HOME ---
    if let Ok(java_home) = std::env::var("JAVA_HOME") {
        push_if_valid(&mut found, Path::new(&java_home));
    }

    // --- Windows registry ---
    #[cfg(windows)]
    {
        find_java_windows_registry(&mut found);
    }

    // --- Linux ---
    #[cfg(target_os = "linux")]
    {
        find_java_linux(&mut found);
    }

    // --- macOS ---
    #[cfg(target_os = "macos")]
    {
        find_java_macos(&mut found);
    }

    // Проверка, что Java реально запускается
    found
        .into_iter()
        .filter(|p| is_java_working(p))
        .map(|p| JavaRuntime {
            path: p.to_string_lossy().to_string(),
        })
        .collect()
}

fn which_java() -> Result<PathBuf, ()> {
    let output = Command::new("which").arg("java").output().map_err(|_| ())?;
    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if path.is_empty() {
        Err(())
    } else {
        Ok(PathBuf::from(path))
    }
}

fn is_java_working(path: &Path) -> bool {
    Command::new(path)
        .arg("-version")
        .output()
        .is_ok()
}

fn push_if_valid(set: &mut HashSet<PathBuf>, java_home: &Path) {
    let bin = if cfg!(windows) {
        java_home.join("bin/javaw.exe")
    } else {
        java_home.join("bin/java")
    };

    if bin.exists() {
        set.insert(bin);
    }
}

#[cfg(windows)]
fn find_java_windows_registry(set: &mut HashSet<PathBuf>) {
    use winreg::{enums::*, RegKey};

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let keys = [
        "SOFTWARE\\JavaSoft\\Java Runtime Environment",
        "SOFTWARE\\JavaSoft\\Java Development Kit",
        "SOFTWARE\\WOW6432Node\\JavaSoft\\Java Runtime Environment",
        "SOFTWARE\\WOW6432Node\\JavaSoft\\Java Development Kit",
    ];

    for key_path in keys {
        if let Ok(key) = hklm.open_subkey(key_path) {
            for version in key.enum_keys().flatten() {
                if let Ok(sub) = key.open_subkey(version) {
                    if let Ok(home) = sub.get_value::<String, _>("JavaHome") {
                        push_if_valid(set, Path::new(&home));
                    }
                }
            }
        }
    }
}

#[cfg(target_os = "linux")]
fn find_java_linux(set: &mut HashSet<PathBuf>) {
    if let Ok(entries) = std::fs::read_dir("/usr/lib/jvm") {
        for e in entries.flatten() {
            push_if_valid(set, &e.path());
        }
    }
}

#[cfg(target_os = "macos")]
fn find_java_macos(set: &mut HashSet<PathBuf>) {
    let out = Command::new("/usr/libexec/java_home")
        .arg("-V")
        .output();

    if let Ok(output) = out {
        let text = String::from_utf8_lossy(&output.stderr);
        for line in text.lines() {
            if let Some(pos) = line.find("/Library/Java/JavaVirtualMachines") {
                let path = &line[pos..];
                let home = Path::new(path).join("Contents/Home");
                push_if_valid(set, &home);
            }
        }
    }
}