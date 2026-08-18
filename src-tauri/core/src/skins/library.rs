//! `<root>/skins/<sha1>.png` + `library.json`.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};

use crate::error::{CommandError, CommandResult};
use crate::fs_util::{ensure_dir, read_json_opt, write_atomic, write_json_atomic};
use crate::icons::to_data_url;

use super::texture::{self, SkinVariant};

const INDEX_FILE: &str = "library.json";
const REMOTE_DIR: &str = "remote";
const MAX_NAME: usize = 48;
const COPY_SUFFIX: &str = "копия";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SkinSource {
    Profile,
    #[default]
    Local,
    Player,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkinEntry {
    pub id: String,
    #[serde(default)]
    pub texture: String,
    pub name: String,
    #[serde(default)]
    pub variant: SkinVariant,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cape_id: Option<String>,
    #[serde(default)]
    pub source: SkinSource,
    #[serde(default)]
    pub added_at: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkinLibrary {
    #[serde(default)]
    pub skins: Vec<SkinEntry>,
}

impl SkinLibrary {
    pub fn find(&self, id: &str) -> Option<&SkinEntry> {
        self.skins.iter().find(|entry| entry.id == id)
    }

    fn find_mut(&mut self, id: &str) -> CommandResult<&mut SkinEntry> {
        self.skins
            .iter_mut()
            .find(|entry| entry.id == id)
            .ok_or_else(|| CommandError::fs("Набор не найден в библиотеке"))
    }

    fn users_of(&self, texture: &str) -> usize {
        self.skins.iter().filter(|entry| entry.texture == texture).count()
    }
}

pub fn index_file(dir: &Path) -> PathBuf {
    dir.join(INDEX_FILE)
}

pub fn texture_file(dir: &Path, texture: &str) -> CommandResult<PathBuf> {
    if texture.is_empty() || !texture.chars().all(|symbol| symbol.is_ascii_hexdigit()) {
        return Err(CommandError::fs(format!("Некорректный идентификатор текстуры: {texture}")));
    }

    Ok(dir.join(format!("{texture}.png")))
}

pub async fn load(dir: &Path) -> SkinLibrary {
    let mut library: SkinLibrary = read_json_opt(&index_file(dir)).await.unwrap_or_default();

    for entry in &mut library.skins {
        if entry.texture.is_empty() {
            entry.texture = entry.id.clone();
        }
    }

    library
}

pub async fn save(dir: &Path, library: &SkinLibrary) -> CommandResult<()> {
    ensure_dir(dir).await?;
    write_json_atomic(&index_file(dir), library).await
}

pub async fn add(
    dir: &Path,
    name: &str,
    bytes: &[u8],
    source: SkinSource,
    variant: Option<SkinVariant>,
) -> CommandResult<SkinEntry> {
    let (normalized, detected) = texture::normalize(bytes)?;

    let texture: String = Sha1::digest(&normalized)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();

    ensure_dir(dir).await?;
    write_atomic(&texture_file(dir, &texture)?, &normalized).await?;

    let mut library = load(dir).await;

    let existing = library
        .skins
        .iter_mut()
        .find(|entry| entry.texture == texture);

    let id = match existing {
        Some(entry) => {
            if source == SkinSource::Profile {
                entry.source = SkinSource::Profile;
            }

            if let Some(variant) = variant {
                entry.variant = variant;
            }

            entry.id.clone()
        }
        None => {
            let entry = SkinEntry {
                id: uuid::Uuid::new_v4().simple().to_string(),
                texture: texture.clone(),
                name: clean_name(name),
                variant: variant.unwrap_or(detected),
                cape_id: None,
                source,
                added_at: now_millis(),
            };

            let id = entry.id.clone();
            library.skins.push(entry);

            id
        }
    };

    save(dir, &library).await?;

    library
        .find(&id)
        .cloned()
        .ok_or_else(|| CommandError::fs("Набор не найден в библиотеке"))
}

pub async fn duplicate(
    dir: &Path,
    id: &str,
    cape_id: Option<String>,
) -> CommandResult<SkinEntry> {
    let mut library = load(dir).await;

    let source = library
        .find(id)
        .cloned()
        .ok_or_else(|| CommandError::fs("Набор не найден в библиотеке"))?;

    let copy = SkinEntry {
        id: uuid::Uuid::new_v4().simple().to_string(),
        name: copy_name(&source.name),
        cape_id,
        added_at: now_millis(),
        ..source
    };

    library.skins.push(copy.clone());

    save(dir, &library).await?;

    Ok(copy)
}

pub async fn rename(dir: &Path, id: &str, name: &str) -> CommandResult<SkinLibrary> {
    let mut library = load(dir).await;

    library.find_mut(id)?.name = clean_name(name);

    save(dir, &library).await?;

    Ok(library)
}

pub async fn set_variant(dir: &Path, id: &str, variant: SkinVariant) -> CommandResult<SkinLibrary> {
    let mut library = load(dir).await;

    library.find_mut(id)?.variant = variant;

    save(dir, &library).await?;

    Ok(library)
}

pub async fn set_cape(dir: &Path, id: &str, cape_id: Option<String>) -> CommandResult<SkinLibrary> {
    let mut library = load(dir).await;

    library.find_mut(id)?.cape_id = cape_id;

    save(dir, &library).await?;

    Ok(library)
}

pub async fn remove(dir: &Path, id: &str) -> CommandResult<SkinLibrary> {
    let mut library = load(dir).await;

    let Some(removed) = library.find(id).cloned() else {
        return Ok(library);
    };

    library.skins.retain(|entry| entry.id != id);

    save(dir, &library).await?;

    if library.users_of(&removed.texture) == 0 {
        if let Ok(path) = texture_file(dir, &removed.texture) {
            crate::fs_util::remove_file_if_exists(&path).await;
        }
    }

    Ok(library)
}

pub async fn read(dir: &Path, texture: &str) -> CommandResult<Vec<u8>> {
    let path = texture_file(dir, texture)?;

    tokio::fs::read(&path)
        .await
        .map_err(|e| CommandError::io("Не удалось прочитать скин", &path, e))
}

pub async fn data_url(dir: &Path, texture: &str) -> CommandResult<String> {
    Ok(to_data_url("image/png", &read(dir, texture).await?))
}

pub async fn remote_bytes(dir: &Path, url: &str) -> CommandResult<Vec<u8>> {
    let cache = dir.join(REMOTE_DIR);
    let key: String = Sha1::digest(url.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();

    let path = cache.join(format!("{key}.png"));

    if let Ok(bytes) = tokio::fs::read(&path).await {
        if !bytes.is_empty() {
            return Ok(bytes);
        }
    }

    let bytes = super::mojang::download(url).await?;

    ensure_dir(&cache).await?;
    write_atomic(&path, &bytes).await?;

    Ok(bytes)
}

pub async fn remote_data_url(dir: &Path, url: &str) -> CommandResult<String> {
    Ok(to_data_url("image/png", &remote_bytes(dir, url).await?))
}

fn clean_name(name: &str) -> String {
    let name: String = name
        .trim()
        .chars()
        .filter(|symbol| !symbol.is_control())
        .take(MAX_NAME)
        .collect();

    let name = name.trim().to_string();

    if name.is_empty() {
        "Без названия".to_string()
    } else {
        name
    }
}

fn copy_name(name: &str) -> String {
    clean_name(&format!("{name} - {COPY_SUFFIX}"))
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| since.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skins::texture::Texture;
    use serde_json::json;

    fn temp_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("cast-skins-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn skin_png(mark: [u8; 4]) -> Vec<u8> {
        let mut texture = Texture::blank(64, 64);

        for y in 0..64 {
            for x in 0..64 {
                texture.set_pixel(x, y, [30, 40, 50, 255]);
            }
        }

        texture.set_pixel(1, 1, mark);
        texture::encode(&texture).unwrap()
    }

    #[tokio::test]
    async fn the_same_texture_never_lands_twice_on_its_own() {
        let dir = temp_dir();

        let one = add(&dir, "Первый", &skin_png([1, 1, 1, 255]), SkinSource::Local, None)
            .await
            .unwrap();
        let two = add(&dir, "Второй", &skin_png([1, 1, 1, 255]), SkinSource::Local, None)
            .await
            .unwrap();

        assert_eq!(one.id, two.id);
        assert_eq!(two.name, "Первый");
        assert_eq!(load(&dir).await.skins.len(), 1);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn duplicating_keeps_the_texture_and_takes_the_given_cape() {
        let dir = temp_dir();

        let origin = add(&dir, "Ночной", &skin_png([2, 2, 2, 255]), SkinSource::Local, None)
            .await
            .unwrap();

        let copy = duplicate(&dir, &origin.id, Some("cape-1".into())).await.unwrap();

        assert_ne!(copy.id, origin.id);
        assert_eq!(copy.texture, origin.texture);
        assert_eq!(copy.variant, origin.variant);
        assert_eq!(copy.cape_id.as_deref(), Some("cape-1"));
        assert_eq!(copy.name, "Ночной - копия");
        assert_eq!(origin.cape_id, None);

        assert_eq!(load(&dir).await.skins.len(), 2);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn deleting_a_copy_keeps_the_texture_for_the_original() {
        let dir = temp_dir();

        let origin = add(&dir, "a", &skin_png([3, 3, 3, 255]), SkinSource::Local, None)
            .await
            .unwrap();
        let copy = duplicate(&dir, &origin.id, None).await.unwrap();

        let library = remove(&dir, &copy.id).await.unwrap();

        assert_eq!(library.skins.len(), 1);
        assert!(texture_file(&dir, &origin.texture).unwrap().exists());

        let library = remove(&dir, &origin.id).await.unwrap();

        assert!(library.skins.is_empty());
        assert!(!texture_file(&dir, &origin.texture).unwrap().exists());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn a_cape_belongs_to_the_entry_not_to_the_texture() {
        let dir = temp_dir();

        let origin = add(&dir, "a", &skin_png([4, 4, 4, 255]), SkinSource::Local, None)
            .await
            .unwrap();
        let copy = duplicate(&dir, &origin.id, Some("cape-1".into())).await.unwrap();

        let library = set_cape(&dir, &origin.id, Some("cape-2".into())).await.unwrap();

        assert_eq!(library.find(&origin.id).unwrap().cape_id.as_deref(), Some("cape-2"));
        assert_eq!(library.find(&copy.id).unwrap().cape_id.as_deref(), Some("cape-1"));

        let library = set_cape(&dir, &copy.id, None).await.unwrap();
        assert_eq!(library.find(&copy.id).unwrap().cape_id, None);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn old_libraries_keep_working_when_the_id_was_the_texture_hash() {
        let dir = temp_dir();

        std::fs::write(
            index_file(&dir),
            serde_json::to_vec(&json!({
                "skins": [{
                    "id": "abc123",
                    "name": "Старый",
                    "variant": "SLIM",
                    "source": "local",
                    "addedAt": 1
                }]
            }))
            .unwrap(),
        )
        .unwrap();

        let library = load(&dir).await;

        assert_eq!(library.skins[0].texture, "abc123");
        assert_eq!(library.skins[0].cape_id, None);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn renaming_trims_and_never_leaves_an_empty_name() {
        let dir = temp_dir();

        let entry = add(&dir, "  ", &skin_png([5, 5, 5, 255]), SkinSource::Local, None).await.unwrap();
        assert_eq!(entry.name, "Без названия");

        let library = rename(&dir, &entry.id, "  Ночной  ").await.unwrap();
        assert_eq!(library.skins[0].name, "Ночной");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn profile_source_wins_over_local() {
        let dir = temp_dir();

        add(&dir, "a", &skin_png([6, 6, 6, 255]), SkinSource::Local, None).await.unwrap();
        let entry = add(&dir, "a", &skin_png([6, 6, 6, 255]), SkinSource::Profile, None)
            .await
            .unwrap();

        assert_eq!(entry.source, SkinSource::Profile);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn texture_paths_reject_anything_but_a_hash() {
        let dir = Path::new("/skins");

        assert!(texture_file(dir, "../accounts.json").is_err());
        assert!(texture_file(dir, "").is_err());
        assert!(texture_file(dir, "abc123").is_ok());
    }
}
