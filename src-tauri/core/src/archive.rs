use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};

use zip::ZipArchive;

use crate::error::{CommandError, CommandResult};

pub async fn extract_natives(jar_path: PathBuf, output_dir: PathBuf) -> CommandResult<()> {
    tokio::task::spawn_blocking(move || extract_natives_blocking(&jar_path, &output_dir))
        .await
        .map_err(|e| CommandError::task_panicked("распаковка нативных библиотек", e))?
}

fn extract_natives_blocking(jar_path: &Path, output_dir: &Path) -> CommandResult<()> {
    let mut archive = open(jar_path)?;

    std::fs::create_dir_all(output_dir)
        .map_err(|e| CommandError::io("Не удалось создать каталог нативов", output_dir, e))?;

    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|e| {
            CommandError::archive(format!("Не удалось прочитать запись архива: {}", jar_path.display()))
                .with_details(e.to_string())
        })?;

        let name = entry.name().to_string();

        if name.ends_with('/') || name.starts_with("META-INF/") || !is_native(&name) {
            continue;
        }

        let Some(file_name) = Path::new(&name).file_name() else { continue };
        let out_path = output_dir.join(file_name);

        let mut out = File::create(&out_path)
            .map_err(|e| CommandError::io("Не удалось создать файл", &out_path, e))?;

        io::copy(&mut entry, &mut out)
            .map_err(|e| CommandError::io("Не удалось распаковать файл", &out_path, e))?;
    }

    Ok(())
}

fn is_native(name: &str) -> bool {
    let lowercase = name.to_ascii_lowercase();
    [".dll", ".so", ".dylib", ".jnilib"]
        .iter()
        .any(|extension| lowercase.ends_with(extension))
        || lowercase.contains(".so.")
}

fn open(jar_path: &Path) -> CommandResult<ZipArchive<File>> {
    let file = File::open(jar_path)
        .map_err(|e| CommandError::io("Не удалось открыть архив", jar_path, e))?;

    ZipArchive::new(file).map_err(|e| {
        CommandError::archive(format!("Повреждённый архив: {}", jar_path.display()))
            .with_details(e.to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_native_libraries() {
        assert!(is_native("windows/x64/lwjgl.dll"));
        assert!(is_native("liblwjgl.so"));
        assert!(is_native("liblwjgl.dylib"));
        assert!(is_native("liblwjgl.jnilib"));
        assert!(is_native("libfoo.so.1"));
        assert!(is_native("LWJGL.DLL"));
    }

    #[test]
    fn ignores_classes_and_metadata() {
        assert!(!is_native("org/lwjgl/Sys.class"));
        assert!(!is_native("module-info.class"));
        assert!(!is_native("LICENSE"));
    }
}
