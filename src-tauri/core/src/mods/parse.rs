use std::collections::HashMap;
use std::io::Read;
use std::path::Path;

use serde_json::Value;

use crate::archive;
use crate::error::CommandResult;

use super::{ModDetails, ModLoader};

const MAX_META: u64 = 1024 * 1024;
const MAX_ICON: u64 = 4 * 1024 * 1024;

const NEOFORGE_TOML: &str = "META-INF/neoforge.mods.toml";
const FORGE_TOML: &str = "META-INF/mods.toml";
const QUILT_JSON: &str = "quilt.mod.json";
const FABRIC_JSON: &str = "fabric.mod.json";
const MCMOD_INFO: &str = "mcmod.info";
const LITEMOD_JSON: &str = "litemod.json";
const MANIFEST: &str = "META-INF/MANIFEST.MF";

const SOURCES: &[(&str, ModLoader)] = &[
    (NEOFORGE_TOML, ModLoader::NeoForge),
    (FORGE_TOML, ModLoader::Forge),
    (QUILT_JSON, ModLoader::Quilt),
    (FABRIC_JSON, ModLoader::Fabric),
    (MCMOD_INFO, ModLoader::Forge),
    (LITEMOD_JSON, ModLoader::LiteLoader),
];

const JAR_VERSION: &str = "${file.jarVersion}";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawIcon {
    pub extension: &'static str,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parsed {
    pub details: ModDetails,
    pub icon: Option<RawIcon>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct Meta {
    pub details: ModDetails,
    pub icon: String,
}

pub fn jar(path: &Path, prefer: Option<ModLoader>) -> CommandResult<Parsed> {
    let mut archive = archive::open(path)?;
    let mut found: HashMap<&'static str, Vec<u8>> = HashMap::new();

    for index in 0..archive.len() {
        let Ok(mut entry) = archive.by_index(index) else { continue };

        let Some(known) = known_entry(entry.name()) else { continue };

        if entry.size() > MAX_META || found.contains_key(known) {
            continue;
        }

        let mut bytes = Vec::with_capacity(entry.size() as usize);

        if entry.read_to_end(&mut bytes).is_ok() {
            found.insert(known, bytes);
        }
    }

    let jar_version = found.get(MANIFEST).and_then(|bytes| manifest_version(bytes));
    let mut meta = pick(&found, prefer, jar_version.as_deref());

    if meta.details.loaders.is_empty() {
        meta.details.loaders = loaders_in(&found);
    }

    let icon = read_icon(&mut archive, &meta.icon);

    Ok(Parsed {
        details: meta.details,
        icon,
    })
}

pub fn folder(path: &Path, prefer: Option<ModLoader>) -> CommandResult<Parsed> {
    let mut found: HashMap<&'static str, Vec<u8>> = HashMap::new();

    for (name, _) in SOURCES.iter().chain(std::iter::once(&(MANIFEST, ModLoader::Forge))) {
        let file = name.split('/').fold(path.to_path_buf(), |acc, part| acc.join(part));

        let Ok(metadata) = std::fs::metadata(&file) else { continue };

        if !metadata.is_file() || metadata.len() > MAX_META {
            continue;
        }

        if let Ok(bytes) = std::fs::read(&file) {
            found.insert(name, bytes);
        }
    }

    let jar_version = found.get(MANIFEST).and_then(|bytes| manifest_version(bytes));
    let mut meta = pick(&found, prefer, jar_version.as_deref());

    if meta.details.loaders.is_empty() {
        meta.details.loaders = loaders_in(&found);
    }

    let icon = crate::fs_util::relative_key(&meta.icon)
        .ok()
        .map(|key| key.split('/').fold(path.to_path_buf(), |acc, part| acc.join(part)))
        .filter(|icon| icon.is_file())
        .and_then(|icon| std::fs::read(&icon).ok())
        .filter(|bytes| bytes.len() as u64 <= MAX_ICON)
        .and_then(|bytes| image_extension(&bytes).map(|extension| RawIcon { extension, bytes }));

    Ok(Parsed {
        details: meta.details,
        icon,
    })
}

fn known_entry(name: &str) -> Option<&'static str> {
    let name = name.trim_start_matches("./").trim_start_matches('/');

    SOURCES
        .iter()
        .map(|(file, _)| *file)
        .chain(std::iter::once(MANIFEST))
        .find(|known| *known == name)
}

pub(super) fn pick(
    found: &HashMap<&'static str, Vec<u8>>,
    prefer: Option<ModLoader>,
    jar_version: Option<&str>,
) -> Meta {
    let mut order: Vec<&'static str> = preferred(prefer);

    for (file, _) in SOURCES {
        if !order.contains(file) {
            order.push(file);
        }
    }

    for file in order {
        let Some(bytes) = found.get(file) else { continue };

        let Some(mut meta) = read(file, bytes) else { continue };

        if meta.details.version == JAR_VERSION {
            meta.details.version = jar_version.unwrap_or_default().to_string();
        }

        if !meta.details.name.trim().is_empty() || !meta.details.mod_id.trim().is_empty() {
            return meta;
        }
    }

    Meta::default()
}

fn preferred(loader: Option<ModLoader>) -> Vec<&'static str> {
    match loader {
        Some(ModLoader::Fabric) => vec![FABRIC_JSON, QUILT_JSON],
        Some(ModLoader::Quilt) => vec![QUILT_JSON, FABRIC_JSON],
        Some(ModLoader::Forge) => vec![FORGE_TOML, NEOFORGE_TOML, MCMOD_INFO],
        Some(ModLoader::NeoForge) => vec![NEOFORGE_TOML, FORGE_TOML, MCMOD_INFO],
        Some(ModLoader::LiteLoader) => vec![LITEMOD_JSON],
        None => Vec::new(),
    }
}

fn loaders_in(found: &HashMap<&'static str, Vec<u8>>) -> Vec<ModLoader> {
    let mut loaders = Vec::new();

    for (file, loader) in SOURCES {
        if found.contains_key(file) && !loaders.contains(loader) {
            loaders.push(*loader);
        }
    }

    loaders
}

fn read(file: &str, bytes: &[u8]) -> Option<Meta> {
    match file {
        NEOFORGE_TOML => mods_toml(bytes, ModLoader::NeoForge),
        FORGE_TOML => mods_toml(bytes, ModLoader::Forge),
        QUILT_JSON => quilt(bytes),
        FABRIC_JSON => fabric(bytes),
        MCMOD_INFO => mcmod_info(bytes),
        LITEMOD_JSON => litemod(bytes),
        _ => None,
    }
}

// https://docs.neoforged.net/docs/gettingstarted/modfiles
pub(super) fn mods_toml(bytes: &[u8], loader: ModLoader) -> Option<Meta> {
    let table: toml::Table = std::str::from_utf8(bytes).ok()?.parse().ok()?;

    let first = table.get("mods")?.as_array()?.first()?.as_table()?;

    let either = |key: &str| -> String {
        first
            .get(key)
            .or_else(|| table.get(key))
            .and_then(toml::Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_string()
    };

    let authors = either("authors");

    let details = ModDetails {
        mod_id: either("modId"),
        name: either("displayName"),
        version: either("version"),
        description: either("description"),
        authors: split_authors(&authors),
        homepage: homepage(&either("displayURL")),
        license: either("license"),
        loaders: vec![loader],
        icon_key: None,
    };

    Some(Meta {
        details,
        icon: either("logoFile"),
    })
}

// https://fabricmc.net/wiki/documentation:fabric_mod_json
pub(super) fn fabric(bytes: &[u8]) -> Option<Meta> {
    let value: Value = serde_json::from_slice(bytes).ok()?;
    let object = value.as_object()?;

    let details = ModDetails {
        mod_id: text(object.get("id")),
        name: text(object.get("name")),
        version: text(object.get("version")),
        description: text(object.get("description")),
        authors: people(object.get("authors")),
        homepage: homepage(&text(value.pointer("/contact/homepage"))),
        license: list(object.get("license")).join(", "),
        loaders: vec![ModLoader::Fabric],
        icon_key: None,
    };

    Some(Meta {
        details,
        icon: icon_path(object.get("icon")),
    })
}

// https://github.com/QuiltMC/rfcs/blob/main/specification/0002-quilt.mod.json.md
pub(super) fn quilt(bytes: &[u8]) -> Option<Meta> {
    let value: Value = serde_json::from_slice(bytes).ok()?;
    let loader = value.get("quilt_loader")?;
    let metadata = loader.get("metadata");

    let field = |key: &str| metadata.and_then(|meta| meta.get(key));

    let details = ModDetails {
        mod_id: text(loader.get("id")),
        name: text(field("name")),
        version: text(loader.get("version")),
        description: text(field("description")),
        authors: people(field("contributors")),
        homepage: homepage(&text(
            metadata.and_then(|meta| meta.pointer("/contact/homepage")),
        )),
        license: list(field("license")).join(", "),
        loaders: vec![ModLoader::Quilt],
        icon_key: None,
    };

    Some(Meta {
        details,
        icon: icon_path(field("icon")),
    })
}

pub(super) fn mcmod_info(bytes: &[u8]) -> Option<Meta> {
    let value: Value = serde_json::from_slice(bytes).ok()?;

    let first = value
        .as_array()
        .or_else(|| value.get("modList").and_then(Value::as_array))?
        .first()?;

    let details = ModDetails {
        mod_id: text(first.get("modid")),
        name: text(first.get("name")),
        version: text(first.get("version")),
        description: text(first.get("description")),
        authors: people(first.get("authorList").or_else(|| first.get("authors"))),
        homepage: homepage(&text(first.get("url"))),
        license: String::new(),
        loaders: vec![ModLoader::Forge],
        icon_key: None,
    };

    Some(Meta {
        details,
        icon: text(first.get("logoFile")),
    })
}

pub(super) fn litemod(bytes: &[u8]) -> Option<Meta> {
    let value: Value = serde_json::from_slice(bytes).ok()?;

    let details = ModDetails {
        mod_id: text(value.get("name")),
        name: text(value.get("name")),
        version: text(value.get("version")),
        description: text(value.get("description")),
        authors: people(value.get("author")),
        homepage: String::new(),
        license: String::new(),
        loaders: vec![ModLoader::LiteLoader],
        icon_key: None,
    };

    Some(Meta {
        details,
        icon: String::new(),
    })
}

pub fn from_file_name(file_name: &str) -> (String, String) {
    let stem = file_name
        .trim_end_matches(super::DISABLED_SUFFIX)
        .rsplit_once('.')
        .map(|(stem, _)| stem)
        .unwrap_or(file_name);

    for (index, byte) in stem.bytes().enumerate() {
        if byte != b'-' && byte != b'_' {
            continue;
        }

        let rest = &stem[index + 1..];

        if index > 0 && rest.starts_with(|c: char| c.is_ascii_digit()) {
            return (stem[..index].to_string(), rest.to_string());
        }
    }

    (stem.to_string(), String::new())
}

fn read_icon(archive: &mut zip::ZipArchive<std::fs::File>, entry: &str) -> Option<RawIcon> {
    let entry = entry.trim().trim_start_matches("./").trim_start_matches('/');

    if entry.is_empty() {
        return None;
    }

    let mut file = archive.by_name(entry).ok()?;

    if file.size() > MAX_ICON {
        return None;
    }

    let mut bytes = Vec::with_capacity(file.size() as usize);
    file.read_to_end(&mut bytes).ok()?;

    image_extension(&bytes).map(|extension| RawIcon { extension, bytes })
}

pub(super) fn image_extension(bytes: &[u8]) -> Option<&'static str> {
    const PNG: &[u8] = b"\x89PNG\r\n\x1a\n";
    const JPEG: &[u8] = b"\xff\xd8\xff";
    const GIF: &[u8] = b"GIF8";

    if bytes.starts_with(PNG) {
        return Some("png");
    }

    if bytes.starts_with(JPEG) {
        return Some("jpg");
    }

    if bytes.starts_with(GIF) {
        return Some("gif");
    }

    if bytes.len() > 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP" {
        return Some("webp");
    }

    None
}

fn manifest_version(bytes: &[u8]) -> Option<String> {
    let text = std::str::from_utf8(bytes).ok()?;

    text.lines()
        .filter_map(|line| line.split_once(':'))
        .find(|(key, _)| key.trim().eq_ignore_ascii_case("Implementation-Version"))
        .map(|(_, value)| value.trim().to_string())
        .filter(|version| !version.is_empty())
}

fn text(value: Option<&Value>) -> String {
    value.and_then(Value::as_str).unwrap_or_default().trim().to_string()
}

fn list(value: Option<&Value>) -> Vec<String> {
    match value {
        Some(Value::String(single)) => vec![single.trim().to_string()],
        Some(Value::Array(items)) => items
            .iter()
            .map(|item| text(Some(item)))
            .filter(|item| !item.is_empty())
            .collect(),
        _ => Vec::new(),
    }
}

fn people(value: Option<&Value>) -> Vec<String> {
    match value {
        Some(Value::String(single)) => split_authors(single),
        Some(Value::Object(map)) => map.keys().map(|key| key.trim().to_string()).collect(),
        Some(Value::Array(items)) => items
            .iter()
            .map(|item| match item {
                Value::String(name) => name.trim().to_string(),
                other => text(other.get("name")),
            })
            .filter(|name| !name.is_empty())
            .collect(),
        _ => Vec::new(),
    }
}

fn split_authors(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|author| !author.is_empty())
        .map(str::to_string)
        .collect()
}

fn icon_path(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(path)) => path.trim().to_string(),
        Some(Value::Object(map)) => map
            .iter()
            .filter_map(|(size, path)| Some((size.parse::<u32>().ok()?, path.as_str()?)))
            .max_by_key(|(size, _)| *size)
            .map(|(_, path)| path.trim().to_string())
            .unwrap_or_default(),
        _ => String::new(),
    }
}

fn homepage(value: &str) -> String {
    let value = value.trim();

    if value.is_empty() || value.starts_with("http://") || value.starts_with("https://") {
        return value.to_string();
    }

    format!("https://{value}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn bytes(value: serde_json::Value) -> Vec<u8> {
        serde_json::to_vec(&value).unwrap()
    }

    #[test]
    fn fabric_metadata_becomes_a_mod_card() {
        let meta = fabric(&bytes(json!({
            "id": "sodium",
            "name": "Sodium",
            "version": "0.5.3",
            "description": "Оптимизация рендера",
            "authors": ["JellySquid", {"name": "IMS"}],
            "contact": {"homepage": "modrinth.com/mod/sodium"},
            "license": "LGPL-3.0",
            "icon": "assets/sodium/icon.png"
        })))
        .unwrap();

        assert_eq!(meta.details.name, "Sodium");
        assert_eq!(meta.details.mod_id, "sodium");
        assert_eq!(meta.details.version, "0.5.3");
        assert_eq!(meta.details.authors, vec!["JellySquid", "IMS"]);
        assert_eq!(meta.details.homepage, "https://modrinth.com/mod/sodium");
        assert_eq!(meta.details.loaders, vec![ModLoader::Fabric]);
        assert_eq!(meta.icon, "assets/sodium/icon.png");
    }

    #[test]
    fn the_biggest_icon_wins() {
        let meta = fabric(&bytes(json!({
            "id": "a", "name": "A",
            "icon": {"16": "small.png", "128": "big.png", "64": "mid.png"}
        })))
        .unwrap();

        assert_eq!(meta.icon, "big.png");
    }

    #[test]
    fn quilt_contributors_are_an_object() {
        let meta = quilt(&bytes(json!({
            "quilt_loader": {
                "id": "quilted",
                "version": "1.0.0",
                "metadata": {
                    "name": "Quilted",
                    "contributors": {"zaralX": "Owner"},
                    "contact": {"homepage": "https://example.dev"},
                    "icon": "icon.png"
                }
            }
        })))
        .unwrap();

        assert_eq!(meta.details.name, "Quilted");
        assert_eq!(meta.details.version, "1.0.0");
        assert_eq!(meta.details.authors, vec!["zaralX"]);
        assert_eq!(meta.details.loaders, vec![ModLoader::Quilt]);
    }

    #[test]
    fn forge_reads_fields_from_both_tables() {
        let toml = r#"
            modLoader = "javafml"
            license = "MIT"
            authors = "zaralX, someone"
            displayURL = "example.dev"

            [[mods]]
            modId = "jei"
            version = "15.2.0"
            displayName = "Just Enough Items"
            description = "Рецепты"
            logoFile = "jei.png"
        "#;

        let meta = mods_toml(toml.as_bytes(), ModLoader::Forge).unwrap();

        assert_eq!(meta.details.mod_id, "jei");
        assert_eq!(meta.details.name, "Just Enough Items");
        assert_eq!(meta.details.license, "MIT");
        assert_eq!(meta.details.authors, vec!["zaralX", "someone"]);
        assert_eq!(meta.details.homepage, "https://example.dev");
        assert_eq!(meta.icon, "jei.png");
    }

    #[test]
    fn the_jar_version_placeholder_comes_from_the_manifest() {
        let toml = br#"
            [[mods]]
            modId = "create"
            version = "${file.jarVersion}"
            displayName = "Create"
        "#;

        let manifest = b"Manifest-Version: 1.0\r\nImplementation-Version: 0.5.1f\r\n";

        let found = HashMap::from([
            (FORGE_TOML, toml.to_vec()),
            (MANIFEST, manifest.to_vec()),
        ]);

        let meta = pick(&found, None, manifest_version(manifest).as_deref());

        assert_eq!(meta.details.version, "0.5.1f");
    }

    #[test]
    fn legacy_mcmod_info_still_parses() {
        let meta = mcmod_info(&bytes(json!([{
            "modid": "journeymap",
            "name": "JourneyMap",
            "version": "5.7.1",
            "authorList": ["techbrew"],
            "url": "https://journeymap.info",
            "logoFile": "/assets/journeymap/icon.png"
        }])))
        .unwrap();

        assert_eq!(meta.details.name, "JourneyMap");
        assert_eq!(meta.details.authors, vec!["techbrew"]);
        assert_eq!(meta.icon, "/assets/journeymap/icon.png");

        let wrapped = mcmod_info(&bytes(json!({
            "modListVersion": 2,
            "modList": [{"modid": "old", "name": "Old"}]
        })))
        .unwrap();

        assert_eq!(wrapped.details.name, "Old");
    }

    #[test]
    fn broken_metadata_is_skipped_instead_of_breaking_the_list() {
        assert!(fabric(b"{ not json").is_none());
        assert!(quilt(&bytes(json!({"no_loader": true}))).is_none());
        assert!(mods_toml(b"[[mods]", ModLoader::Forge).is_none());
        assert!(mods_toml(b"modId = \"x\"", ModLoader::Forge).is_none(), "нет [[mods]] - нет мода");
        assert_eq!(pick(&HashMap::new(), None, None), Meta::default());
    }

    #[test]
    fn a_multiloader_jar_shows_the_metadata_of_its_own_loader() {
        let fabric_json = bytes(json!({"id": "arch", "name": "Architectury Fabric", "version": "9.1.0"}));
        let forge_toml = br#"
            [[mods]]
            modId = "arch"
            version = "9.1.1"
            displayName = "Architectury Forge"
        "#;

        let found = HashMap::from([
            (FABRIC_JSON, fabric_json),
            (FORGE_TOML, forge_toml.to_vec()),
        ]);

        assert_eq!(pick(&found, Some(ModLoader::Fabric), None).details.name, "Architectury Fabric");
        assert_eq!(pick(&found, Some(ModLoader::Forge), None).details.name, "Architectury Forge");
    }

    #[test]
    fn a_name_can_be_guessed_from_the_file_name() {
        assert_eq!(
            from_file_name("jei-1.20.1-15.2.0.27.jar"),
            ("jei".to_string(), "1.20.1-15.2.0.27".to_string())
        );
        assert_eq!(
            from_file_name("sodium-fabric-mc1.20.1-0.5.3.jar.disabled"),
            ("sodium-fabric-mc1.20.1".to_string(), "0.5.3".to_string())
        );
        assert_eq!(from_file_name("optifine.jar"), ("optifine".to_string(), String::new()));
    }

    #[test]
    fn only_real_images_are_taken_for_icons() {
        assert_eq!(image_extension(b"\x89PNG\r\n\x1a\nrest"), Some("png"));
        assert_eq!(image_extension(b"\xff\xd8\xffrest"), Some("jpg"));
        assert_eq!(image_extension(b"RIFF____WEBPVP8 "), Some("webp"));
        assert_eq!(image_extension(b"<svg>not really</svg>"), None);
        assert_eq!(image_extension(b""), None);
    }

    #[test]
    fn metadata_file_names_are_matched_exactly() {
        assert_eq!(known_entry("fabric.mod.json"), Some(FABRIC_JSON));
        assert_eq!(known_entry("./META-INF/mods.toml"), Some(FORGE_TOML));
        assert_eq!(known_entry("assets/fabric.mod.json"), None);
        assert_eq!(known_entry("META-INF/services/whatever"), None);
    }
}
