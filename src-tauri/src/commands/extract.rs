use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};
use zip::ZipArchive;

#[tauri::command]
pub fn extract_jar(jar_path: String, output_dir: String) -> Result<(), String> {
    let jar_path = PathBuf::from(jar_path);
    let output_dir = PathBuf::from(output_dir);

    let file = File::open(&jar_path)
        .map_err(|e| format!("Failed to open jar: {e}"))?;

    let mut archive = ZipArchive::new(file)
        .map_err(|e| format!("Invalid zip archive: {e}"))?;

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| format!("Zip entry error: {e}"))?;

        let name = entry.name();

        // пропускаем META-INF и директории
        if name.starts_with("META-INF/") || name.ends_with('/') {
            continue;
        }

        // фильтр нативных файлов
        let is_native = name.ends_with(".dll")
            || name.ends_with(".so")
            || name.ends_with(".dylib");

        if !is_native {
            continue;
        }

        let filename = Path::new(name)
            .file_name()
            .ok_or("Invalid file name in zip")?;

        let out_path = output_dir.join(filename);

        let mut outfile = File::create(&out_path)
            .map_err(|e| format!("Failed to create file {:?}: {e}", out_path))?;

        io::copy(&mut entry, &mut outfile)
            .map_err(|e| format!("Failed to extract {:?}: {e}", out_path))?;
    }

    Ok(())
}