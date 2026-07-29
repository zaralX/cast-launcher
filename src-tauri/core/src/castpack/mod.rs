pub mod catalog;
pub mod manifest;
pub mod mods;
pub mod resolve;
pub mod source;

use crate::error::{CommandError, CommandResult};

pub use catalog::{Catalog, CatalogPack};
pub use manifest::{FileMode, Manifest, ModRef, SeedFile};
pub use resolve::{merge, Overlay};

pub const SCHEMA_VERSION: u32 = 1;

pub fn https_url(url: &str) -> CommandResult<&str> {
    let trimmed = url.trim();

    let parsed = url::Url::parse(trimmed)
        .map_err(|e| CommandError::manifest(format!("Некорректная ссылка: {url}")).with_details(e.to_string()))?;

    if parsed.scheme() != "https" {
        return Err(CommandError::manifest(format!(
            "Лаунчер качает файлы сборок только по https: {url}"
        )));
    }

    if parsed.host_str().is_none() {
        return Err(CommandError::manifest(format!("В ссылке нет адреса сервера: {url}")));
    }

    Ok(trimmed)
}

pub fn host_of(url: &str) -> Option<String> {
    url::Url::parse(url.trim())
        .ok()
        .and_then(|parsed| parsed.host_str().map(str::to_string))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_https_links_are_accepted() {
        assert_eq!(https_url("  https://cdn.zaralx.ru/a.json  ").unwrap(), "https://cdn.zaralx.ru/a.json");

        assert!(https_url("http://cdn.zaralx.ru/a.json").is_err());
        assert!(https_url("file:///C:/evil.jar").is_err());
        assert!(https_url("не ссылка").is_err());
        assert!(https_url("").is_err());
    }

    #[test]
    fn the_host_is_taken_from_the_link() {
        assert_eq!(host_of("https://cdn.zaralx.ru/packs/a.json").as_deref(), Some("cdn.zaralx.ru"));
        assert_eq!(host_of("не ссылка"), None);
    }
}
