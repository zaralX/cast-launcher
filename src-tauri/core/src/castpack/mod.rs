pub mod base;
pub mod catalog;
pub mod export;
pub mod file;
pub mod manifest;
pub mod mods;
pub mod resolve;
pub mod source;

use crate::error::{CommandError, CommandResult};

pub use base::{base_pack, target, Target};
pub use catalog::{Catalog, CatalogPack};
pub use manifest::{EmbeddedFile, FileMode, Manifest, ModRef, Origin, SeedFile};
pub use resolve::{merge, Overlay};

pub const SCHEMA_VERSION: u32 = 1;

pub fn https_url(url: &str) -> CommandResult<&str> {
    let trimmed = url.trim();

    let parsed = url::Url::parse(trimmed).map_err(|e| {
        CommandError::manifest("error.reason.links.invalid")
            .param("url", url)
            .with_details(e.to_string())
    })?;

    if parsed.scheme() != "https" {
        return Err(CommandError::manifest("error.reason.castpack.https_only").param("url", url));
    }

    if parsed.host_str().is_none() {
        return Err(CommandError::manifest("error.reason.links.no_host").param("url", url));
    }

    Ok(trimmed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_https_links_are_accepted() {
        assert_eq!(
            https_url("  https://cdn.zaralx.ru/a.json  ").unwrap(),
            "https://cdn.zaralx.ru/a.json"
        );

        assert!(https_url("http://cdn.zaralx.ru/a.json").is_err());
        assert!(https_url("file:///C:/evil.jar").is_err());
        assert!(https_url("не ссылка").is_err());
        assert!(https_url("").is_err());
    }
}
