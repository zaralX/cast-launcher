fn main() {
    tauri_build::build();

    #[cfg(windows)]
    windows::place_webview2_loader_next_to_tests();
}

#[cfg(windows)]
mod windows {
    use std::path::{Path, PathBuf};

    pub fn place_webview2_loader_next_to_tests() {
        let Ok(out_dir) = std::env::var("OUT_DIR") else { return };
        let out_dir = PathBuf::from(out_dir);

        // OUT_DIR = target/<profile>/build/<crate>-<hash>/out
        let Some(profile_dir) = out_dir.ancestors().nth(3) else { return };

        let Some(source) = find_loader(&profile_dir.join("build")) else {
            println!("cargo:warning=WebView2Loader.dll не найден, cargo test может не запуститься");
            return;
        };

        let deps = profile_dir.join("deps");
        if !deps.is_dir() {
            return;
        }

        let target = deps.join("WebView2Loader.dll");
        if target.exists() {
            return;
        }

        if let Err(error) = std::fs::copy(&source, &target) {
            println!("cargo:warning=Не удалось скопировать WebView2Loader.dll: {error}");
        }
    }

    fn find_loader(build_dir: &Path) -> Option<PathBuf> {
        let arch = match std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() {
            Ok("x86") => "x86",
            Ok("aarch64") => "arm64",
            _ => "x64",
        };

        std::fs::read_dir(build_dir)
            .ok()?
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("webview2-com-sys-")
            })
            .map(|entry| entry.path().join("out").join(arch).join("WebView2Loader.dll"))
            .find(|path| path.is_file())
    }
}
