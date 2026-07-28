use std::collections::BTreeMap;

use crate::error::{CommandError, CommandResult};
use crate::net::meta_cache::MetaCache;

pub const BASE: &str = "https://assets.zaralx.ru/api/v1/minecraft/vanilla";

pub type ItemCategories = BTreeMap<String, Vec<String>>;

pub async fn item_categories(meta: &MetaCache) -> CommandResult<ItemCategories> {
    let categories: ItemCategories = meta.fetch_json(&format!("{BASE}/item/categories")).await?;

    Ok(categories
        .into_iter()
        .map(|(category, items)| {
            let items = items.into_iter().filter(|item| is_item_id(item)).collect();
            (category, items)
        })
        .filter(|(_, items): &(String, Vec<String>)| !items.is_empty())
        .collect())
}

pub async fn item_icon(meta: &MetaCache, item: &str) -> CommandResult<Vec<u8>> {
    meta.fetch_bytes(&item_icon_url(item)?).await
}

pub async fn item_names(meta: &MetaCache, language: &str) -> CommandResult<BTreeMap<String, String>> {
    let translations: BTreeMap<String, String> =
        meta.fetch_json(&format!("{BASE}/lang/{}", language_id(language))).await?;

    let mut names = BTreeMap::new();

    for prefix in ["block.minecraft.", "item.minecraft."] {
        for (key, value) in &translations {
            let Some(id) = key.strip_prefix(prefix) else { continue };

            if is_item_id(id) {
                names.insert(id.to_string(), value.clone());
            }
        }
    }

    Ok(names)
}

pub fn item_icon_url(item: &str) -> CommandResult<String> {
    if !is_item_id(item) {
        return Err(CommandError::fs(format!("Некорректный идентификатор предмета: {item}")));
    }

    Ok(format!("{BASE}/item/{item}/icon"))
}

pub fn item_icon_file(item: &str) -> String {
    format!("mc-{item}.webp")
}

pub fn is_item_id(item: &str) -> bool {
    !item.is_empty()
        && item.len() <= 64
        && item
            .chars()
            .all(|symbol| symbol.is_ascii_lowercase() || symbol.is_ascii_digit() || symbol == '_')
}

fn language_id(language: &str) -> String {
    let language = language.trim().to_lowercase();

    match language.as_str() {
        "" => "en_us".to_string(),
        "en" => "en_us".to_string(),
        other if other.contains('_') => other.to_string(),
        other => format!("{other}_{other}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn item_ids_are_validated_before_they_reach_a_url() {
        assert!(is_item_id("diamond_sword"));
        assert!(is_item_id("music_disc_11"));
        assert!(!is_item_id("../../secret"));
        assert!(!is_item_id("Diamond"));
        assert!(!is_item_id(""));

        assert_eq!(
            item_icon_url("grass_block").unwrap(),
            "https://assets.zaralx.ru/api/v1/minecraft/vanilla/item/grass_block/icon"
        );
        assert!(item_icon_url("a/b").is_err());
    }

    #[test]
    fn catalog_icons_get_a_recognizable_file_name() {
        assert_eq!(item_icon_file("grass_block"), "mc-grass_block.webp");
    }

    #[tokio::test]
    #[ignore = "ходит в сеть"]
    async fn catalog_is_reachable() {
        let dir = std::env::temp_dir().join(format!("cast-assets-{}", uuid::Uuid::new_v4()));
        let meta = MetaCache::new(dir.clone());

        let categories = item_categories(&meta).await.unwrap();
        assert!(categories.contains_key("combat"));
        assert!(categories["combat"].contains(&"diamond_sword".to_string()));

        let icon = item_icon(&meta, "grass_block").await.unwrap();
        assert!(icon.starts_with(b"RIFF"), "ожидали webp");

        let names = item_names(&meta, "ru").await.unwrap();
        assert_eq!(names.get("diamond_sword").map(String::as_str), Some("Алмазный меч"));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn language_codes_are_expanded_to_minecraft_ids() {
        assert_eq!(language_id("ru"), "ru_ru");
        assert_eq!(language_id("EN"), "en_us");
        assert_eq!(language_id("pt_br"), "pt_br");
        assert_eq!(language_id(""), "en_us");
    }
}
