use crate::error::{CommandError, CommandResult};
use crate::instance::{Instance, LoaderType, PackSource};

use super::PackVersion;

/// The modpack source whose version the player wants to change, if it can be changed at all.
pub fn switchable_pack(instance: &Instance) -> CommandResult<PackSource> {
    if instance.castpack.is_some() {
        return Err(CommandError::unsupported(
            "error.reason.castpack.version_locked",
        ));
    }

    instance
        .pack
        .clone()
        .ok_or_else(|| CommandError::unsupported("error.reason.instance.no_pack_versions"))
}

/// Checks the fetched `version` of the `current` pack and builds the source to install.
pub fn switch_to(
    current: PackSource,
    version: PackVersion,
) -> CommandResult<(PackSource, LoaderType, String)> {
    if !version.project_id.is_empty() && version.project_id != current.project_id {
        return Err(CommandError::invalid_input(
            "error.reason.instance.foreign_version",
        ));
    }

    if let Some(reason) = version.unsupported_reason() {
        return Err(
            CommandError::unsupported("error.reason.instance.version_unsupported_because")
                .param("version", &version.version_number)
                .param_text("reason", reason),
        );
    }

    let (Some(loader), Some(minecraft_version), Some(file)) =
        (version.loader, version.minecraft_version, version.file)
    else {
        return Err(
            CommandError::unsupported("error.reason.instance.version_unsupported")
                .param("version", &version.version_number),
        );
    };

    let pack = PackSource {
        provider: current.provider,
        project_id: current.project_id,
        version_id: version.id,
        version_number: version.version_number,
        file_url: file.url,
        file_name: file.filename,
        file_sha1: file.hashes.sha1,
        file_size: file.size,
    };

    Ok((pack, loader, minecraft_version))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorCode;
    use crate::instance::PackProvider;
    use crate::packs::{FileHashes, PackFile};
    use serde_json::json;

    fn instance(extra: serde_json::Value) -> Instance {
        let mut base = json!({
            "id": "abc",
            "name": "Pack",
            "minecraftVersion": "1.20.1",
            "type": "fabric"
        });

        if let (Some(base), Some(extra)) = (base.as_object_mut(), extra.as_object()) {
            for (key, value) in extra {
                base.insert(key.clone(), value.clone());
            }
        }

        serde_json::from_value(base).unwrap()
    }

    fn current() -> PackSource {
        PackSource {
            provider: PackProvider::Modrinth,
            project_id: "pack".into(),
            version_id: "old".into(),
            version_number: "0.9".into(),
            file_url: "https://cdn.modrinth.com/old.mrpack".into(),
            file_name: "old.mrpack".into(),
            file_sha1: None,
            file_size: None,
        }
    }

    fn version(project_id: &str, supported: bool) -> PackVersion {
        PackVersion {
            provider: PackProvider::Modrinth,
            id: "new".into(),
            project_id: project_id.into(),
            name: "1.0".into(),
            version_number: "1.0".into(),
            version_type: "release".into(),
            downloads: 0,
            date_published: None,
            game_versions: vec!["1.20.1".into()],
            loaders: vec!["fabric".into()],
            minecraft_version: Some("1.20.1".into()),
            loader: Some(LoaderType::Fabric),
            file: supported.then(|| PackFile {
                url: "https://cdn.modrinth.com/new.mrpack".into(),
                filename: "new.mrpack".into(),
                size: Some(1),
                hashes: FileHashes::default(),
            }),
            blocked: false,
            supported,
        }
    }

    #[test]
    fn a_manual_instance_has_no_versions_to_switch() {
        let error = switchable_pack(&instance(json!({}))).unwrap_err();

        assert_eq!(error.code, ErrorCode::Unsupported);
        assert!(error.text.mentions("no_pack_versions"));
    }

    #[test]
    fn a_castpack_instance_keeps_its_base_version() {
        let instance = instance(json!({
            "pack": serde_json::to_value(current()).unwrap(),
            "castpack": { "catalogId": "rpg", "manifestUrl": "https://castpacks.zaralx.ru/rpg.json" }
        }));

        assert!(switchable_pack(&instance)
            .unwrap_err()
            .text
            .mentions("version_locked"));
    }

    #[test]
    fn a_version_of_another_pack_is_refused() {
        let error = switch_to(current(), version("other", true)).unwrap_err();

        assert_eq!(error.code, ErrorCode::InvalidInput);
    }

    #[test]
    fn an_unsupported_version_names_the_catalog_reason() {
        let error = switch_to(current(), version("pack", false)).unwrap_err();

        assert_eq!(error.code, ErrorCode::Unsupported);
        assert!(error.text.mentions("catalog.unsupported.archive"));
    }

    #[test]
    fn a_supported_version_keeps_the_project_and_takes_the_new_file() {
        let (pack, loader, minecraft) = switch_to(current(), version("pack", true)).unwrap();

        assert_eq!(pack.project_id, "pack");
        assert_eq!(pack.version_id, "new");
        assert_eq!(pack.file_name, "new.mrpack");
        assert_eq!(loader, LoaderType::Fabric);
        assert_eq!(minecraft, "1.20.1");
    }
}
