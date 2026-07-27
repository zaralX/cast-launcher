use crate::error::CommandError;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::{fs, io};
use zip::ZipArchive;

fn open_archive(jar_path: &Path) -> Result<ZipArchive<File>, CommandError> {
    let file = File::open(jar_path).map_err(|e| {
        CommandError::fs(format!("Не удалось открыть архив: {}", jar_path.display()))
            .with_details(e.to_string())
    })?;

    ZipArchive::new(file).map_err(|e| {
        CommandError::archive(format!("Повреждённый архив: {}", jar_path.display()))
            .with_details(e.to_string())
    })
}

#[tauri::command]
pub fn extract_jar(jar_path: String, output_dir: String) -> Result<(), CommandError> {
    let jar_path = PathBuf::from(jar_path);
    let output_dir = PathBuf::from(output_dir);

    let mut archive = open_archive(&jar_path)?;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| {
            CommandError::archive(format!("Не удалось прочитать запись архива: {}", jar_path.display()))
                .with_details(e.to_string())
        })?;

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
            .ok_or_else(|| {
                CommandError::archive(format!("Некорректное имя файла в архиве: {name}"))
            })?
            .to_owned();

        let out_path = output_dir.join(filename);

        let mut outfile = File::create(&out_path).map_err(|e| {
            CommandError::fs(format!("Не удалось создать файл: {}", out_path.display()))
                .with_details(e.to_string())
        })?;

        io::copy(&mut entry, &mut outfile).map_err(|e| {
            CommandError::fs(format!("Не удалось распаковать: {}", out_path.display()))
                .with_details(e.to_string())
        })?;
    }

    Ok(())
}

#[tauri::command]
pub fn extract_everything_jar(jar_path: String, output_dir: String) -> Result<(), CommandError> {
    let jar_path = PathBuf::from(jar_path);
    let output_dir = PathBuf::from(output_dir);

    let mut archive = open_archive(&jar_path)?;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| {
            CommandError::archive(format!("Не удалось прочитать запись архива: {}", jar_path.display()))
                .with_details(e.to_string())
        })?;

        let entry_path = match entry.enclosed_name() {
            Some(p) => p.to_owned(),
            None => continue, // защита от zip-slip
        };

        let out_path = output_dir.join(entry_path);

        if entry.is_dir() {
            fs::create_dir_all(&out_path).map_err(|e| {
                CommandError::fs(format!("Не удалось создать папку: {}", out_path.display()))
                    .with_details(e.to_string())
            })?;
            continue;
        }

        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                CommandError::fs(format!("Не удалось создать папку: {}", parent.display()))
                    .with_details(e.to_string())
            })?;
        }

        let mut outfile = File::create(&out_path).map_err(|e| {
            CommandError::fs(format!("Не удалось создать файл: {}", out_path.display()))
                .with_details(e.to_string())
        })?;

        io::copy(&mut entry, &mut outfile).map_err(|e| {
            CommandError::fs(format!("Не удалось распаковать: {}", out_path.display()))
                .with_details(e.to_string())
        })?;
    }

    Ok(())
}
