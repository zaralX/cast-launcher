use serde::{Deserialize, Serialize};

use crate::error::{CommandError, CommandResult};
use crate::instance::LoaderType;

use super::{https_url, SCHEMA_VERSION};

pub const DEFAULT_URL: &str = "https://castpacks.zaralx.ru/castpacks.json";

pub const MAX_PACKS: usize = 200;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct CatalogPack {
    pub id: String,
    pub name: String,
    pub summary: String,
    pub description: String,
    pub author: String,
    pub version: String,
    pub manifest: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(default = "yes")]
    pub autoupdate: bool,
    pub minecraft_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loader: Option<LoaderType>,
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub website: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_launcher_version: Option<String>,
}

fn yes() -> bool {
    true
}

impl Default for Catalog {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            updated_at: String::new(),
            packs: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Catalog {
    pub schema_version: u32,
    pub updated_at: String,
    pub packs: Vec<CatalogPack>,
}

impl Catalog {
    pub fn parse(bytes: &[u8]) -> CommandResult<Self> {
        let catalog: Self = serde_json::from_slice(bytes).map_err(|e| {
            CommandError::manifest("error.reason.castpack.catalog_corrupted")
                .with_details(e.to_string())
        })?;

        catalog.validated()
    }

    fn validated(mut self) -> CommandResult<Self> {
        if self.schema_version != SCHEMA_VERSION {
            return Err(
                CommandError::unsupported("error.reason.castpack.catalog_version")
                    .param("version", self.schema_version)
                    .param("supported", SCHEMA_VERSION),
            );
        }

        self.packs.truncate(MAX_PACKS);
        self.packs.retain(|pack| match pack.check() {
            Ok(()) => true,
            Err(error) => {
                log::warn!("Skipping catalog pack '{}': {}", pack.id, error);
                false
            }
        });

        Ok(self)
    }

    pub fn find(&self, id: &str) -> Option<&CatalogPack> {
        self.packs.iter().find(|pack| pack.id == id)
    }
}

impl CatalogPack {
    fn check(&self) -> CommandResult<()> {
        if !is_safe_id(&self.id) {
            return Err(
                CommandError::manifest("error.reason.castpack.invalid_id").param("id", &self.id)
            );
        }

        if self.name.trim().is_empty() {
            return Err(CommandError::manifest("error.reason.castpack.no_name"));
        }

        https_url(&self.manifest)?;

        if let Some(icon) = &self.icon {
            https_url(icon)?;
        }

        Ok(())
    }

    pub fn instance_id(&self) -> String {
        format!("castpack-{}", self.id)
    }

    /// The short text on the catalog card, falling back to the long description.
    pub fn card_description(&self) -> String {
        match self.summary.trim().is_empty() {
            true => self.description.trim().to_string(),
            false => self.summary.trim().to_string(),
        }
    }

    pub fn icon_file_name(&self, url: &str) -> String {
        format!("castpack-{}.{}", self.id, crate::packs::icon_extension(url))
    }
}

pub fn is_safe_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .chars()
            .all(|symbol| symbol.is_ascii_alphanumeric() || symbol == '-' || symbol == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_names_carry_the_pack_id_and_keep_the_extension() {
        let pack = CatalogPack {
            id: "zaralx-rpg".into(),
            ..Default::default()
        };

        assert_eq!(
            pack.icon_file_name("https://cdn.zaralx.ru/icons/rpg.WEBP"),
            "castpack-zaralx-rpg.webp"
        );
        assert_eq!(
            pack.icon_file_name("https://cdn.zaralx.ru/icons/rpg.png?v=2"),
            "castpack-zaralx-rpg.png"
        );
        assert_eq!(
            pack.icon_file_name("https://cdn.zaralx.ru/icons/rpg"),
            "castpack-zaralx-rpg.png",
            "no extension means png"
        );
    }

    #[test]
    fn the_card_description_falls_back_to_the_long_text() {
        let mut entry = CatalogPack {
            summary: "  ".into(),
            description: "  Длинное описание  ".into(),
            ..Default::default()
        };

        assert_eq!(entry.card_description(), "Длинное описание");

        entry.summary = "Короткое".into();
        assert_eq!(entry.card_description(), "Короткое");
    }
    use serde_json::json;

    fn catalog(packs: serde_json::Value) -> CommandResult<Catalog> {
        let value = json!({"schemaVersion": SCHEMA_VERSION, "packs": packs});
        Catalog::parse(&serde_json::to_vec(&value).unwrap())
    }

    fn pack() -> serde_json::Value {
        json!({
            "id": "zaralx-rpg",
            "name": "zaralX RPG",
            "version": "1.4.2",
            "manifest": "https://cdn.zaralx.ru/packs/rpg/manifest.json"
        })
    }

    #[test]
    fn a_catalog_of_another_schema_is_rejected() {
        let value = json!({"schemaVersion": 42, "packs": []});
        assert!(Catalog::parse(&serde_json::to_vec(&value).unwrap()).is_err());
    }

    #[test]
    fn broken_json_is_reported_as_a_manifest_problem() {
        assert_eq!(
            Catalog::parse(b"{ not json").unwrap_err().code,
            crate::error::ErrorCode::ManifestInvalid
        );
    }

    #[test]
    fn one_broken_entry_does_not_hide_the_rest_of_the_catalog() {
        let mut broken = pack();
        broken["id"] = json!("../evil");

        let mut without_link = pack();
        without_link["id"] = json!("no-link");
        without_link["manifest"] = json!("http://cdn.zaralx.ru/m.json");

        let parsed = catalog(json!([broken, pack(), without_link])).unwrap();

        assert_eq!(parsed.packs.len(), 1);
        assert_eq!(parsed.packs[0].id, "zaralx-rpg");
    }

    #[test]
    fn identifiers_are_limited_to_what_can_name_a_folder() {
        assert!(is_safe_id("zaralx-rpg"));
        assert!(is_safe_id("rpg_2"));

        assert!(!is_safe_id(""));
        assert!(!is_safe_id("../evil"));
        assert!(!is_safe_id("с кириллицей"));
        assert!(!is_safe_id("C:"));
        assert!(!is_safe_id(&"a".repeat(65)));
    }

    #[test]
    fn autoupdate_is_on_unless_the_catalog_says_otherwise() {
        let parsed = catalog(json!([pack()])).unwrap();
        assert!(parsed.packs[0].autoupdate);

        let mut off = pack();
        off["autoupdate"] = json!(false);

        assert!(!catalog(json!([off])).unwrap().packs[0].autoupdate);
    }

    #[test]
    fn a_pack_is_found_by_its_id_and_names_its_instance() {
        let parsed = catalog(json!([pack()])).unwrap();
        let found = parsed.find("zaralx-rpg").unwrap();

        assert_eq!(found.instance_id(), "castpack-zaralx-rpg");
        assert!(parsed.find("нет такой").is_none());
    }

    #[test]
    fn an_oversized_catalog_is_cut_down() {
        let many: Vec<_> = (0..MAX_PACKS + 10)
            .map(|i| {
                let mut entry = pack();
                entry["id"] = json!(format!("pack-{i}"));
                entry
            })
            .collect();

        assert_eq!(catalog(json!(many)).unwrap().packs.len(), MAX_PACKS);
    }

    #[test]
    fn a_catalog_survives_a_json_round_trip() {
        let mut entry = pack();
        entry["icon"] = json!("https://cdn.zaralx.ru/icons/rpg.png");
        entry["loader"] = json!("forge");
        entry["minecraftVersion"] = json!("1.20.1");

        let parsed = catalog(json!([entry])).unwrap();
        let written = serde_json::to_value(&parsed).unwrap();

        assert_eq!(written["packs"][0]["loader"], "forge");
        assert_eq!(written["packs"][0]["minecraftVersion"], "1.20.1");

        let again: Catalog = serde_json::from_value(written).unwrap();
        assert_eq!(again, parsed);
    }
}
