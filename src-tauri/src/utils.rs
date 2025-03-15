use std::fs::{create_dir_all, File};
use std::{fs, io};
use std::io::Read;
use std::path::{Path, PathBuf};
use semver::Version;
use serde_json::Value;
use zip::ZipArchive;

pub async fn http_get_json(url: &str) -> Value {
    println!("Requesting: {}", url);
    let response = reqwest::get(url)
        .await
        .expect("Error when get version list")
        .text()
        .await
        .unwrap();
    serde_json::from_str(&response).unwrap()
}

pub(crate) fn extract_jar(jar_path: PathBuf, output_dir: PathBuf) -> io::Result<()> {
    let file = File::open(jar_path)?;
    let mut archive = ZipArchive::new(file)?;

    create_dir_all(&output_dir)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let outpath = output_dir.join(file.name());

        if file.name().ends_with('/') {
            create_dir_all(&outpath)?;
        } else {
            if let Some(parent) = outpath.parent() {
                create_dir_all(parent)?;
            }
            let mut outfile = File::create(&outpath)?;
            io::copy(&mut file, &mut outfile)?;
        }
    }
    Ok(())
}

pub fn get_absolute_path(path: PathBuf) -> String {
    let full_path = path.canonicalize().unwrap();

    let full_path_str = full_path.to_str().unwrap();
    full_path_str.strip_prefix(r"\\?\").unwrap_or(full_path_str).to_owned()
}

pub fn is_current_os(target: &str) -> bool {
    let current_os = std::env::consts::OS;
    match (target, current_os) {
        ("osx", "macos") => true, // macOS обозначается как "macos" в Rust
        (x, y) if x == y => true, // Прямое совпадение
        _ => false,
    }
}

pub fn is_current_arch(target: &str) -> bool {
    let current_arch = std::env::consts::ARCH;
    current_arch == target
}

pub fn get_latest_fabric_version(data: &Vec<Value>) -> Option<String> {
    data.iter()
        .filter_map(|entry| {
            entry.get("loader")?.get("version")?.as_str().map(|s| s.to_string())
        })
        .filter_map(|s| Version::parse(&s).ok()) // Парсим версию
        .max() // Находим самую большую версию
        .map(|v| v.to_string()) // Возвращаем строку
}

pub fn generate_maven_url(library: &Value) -> Option<String> {
    let name = library["name"].as_str()?;
    let url = library["url"].as_str().unwrap_or("https://maven.fabricmc.net/");

    let parts: Vec<&str> = name.split(':').collect();
    if parts.len() != 3 {
        return None; // Неправильный формат библиотеки
    }

    let group_id = parts[0].replace('.', "/");
    let artifact_id = parts[1];
    let version = parts[2];

    Some(format!("{url}{group_id}/{artifact_id}/{version}/{artifact_id}-{version}.jar"))
}

pub fn copy_dir_recursive(src: &Path, dest: &Path) -> io::Result<()> {
    if !dest.exists() {
        create_dir_all(dest)?;
    }

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let entry_path = entry.path();
        let dest_path = dest.join(entry.file_name());

        if entry_path.is_dir() {
            copy_dir_recursive(&entry_path, &dest_path)?;
        } else {
            fs::copy(&entry_path, &dest_path)?;
        }
    }

    Ok(())
}

pub fn read_json_file(file_path: PathBuf) -> Value {
    let mut file = File::open(file_path).unwrap();

    // Читаем содержимое
    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap();

    let json = serde_json::from_str(&contents).expect("Error when reading JSON file");
    json
}

pub fn create_unique_dir(base_path: &Path, new_dir: &str) -> std::io::Result<PathBuf> {
    let mut path = Path::new(base_path).join(new_dir);
    let mut count = 1;

    while path.exists() {
        path = Path::new(base_path).join(format!("{} ({})", new_dir, count));
        count += 1;
    }

    fs::create_dir(&path)?;
    Ok(path)
}