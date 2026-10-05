use crate::error::{CommandError, CommandResult};
use crate::instance::{LoaderType, PackProvider, PackSource};
use crate::packs::{self, PackVersion};

use super::Manifest;

/// Resolves the modpack a CastPack builds on, if its manifest names one.
pub async fn base_pack(
    manifest: &Manifest,
) -> CommandResult<Option<(PackSource, LoaderType, String)>> {
    let Some(spec) = &manifest.base else {
        return Ok(None);
    };

    let version = packs::version(spec.provider, &spec.project_id, &spec.version_id).await?;

    resolve(spec.provider, &spec.project_id, version).map(Some)
}

fn resolve(
    provider: PackProvider,
    project_id: &str,
    version: PackVersion,
) -> CommandResult<(PackSource, LoaderType, String)> {
    if let Some(reason) = version.unsupported_reason() {
        return Err(
            CommandError::unsupported("error.reason.castpack.base_pack_unsupported")
                .param_text("reason", reason),
        );
    }

    let (Some(loader), Some(minecraft_version), Some(file)) =
        (version.loader, version.minecraft_version, version.file)
    else {
        return Err(CommandError::manifest("error.reason.castpack.base_no_file"));
    };

    let pack = PackSource {
        provider,
        project_id: project_id.to_string(),
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
    use crate::packs::{FileHashes, PackFile};

    fn version(loader: Option<LoaderType>, file: bool, blocked: bool) -> PackVersion {
        PackVersion {
            provider: PackProvider::Modrinth,
            id: "v1".into(),
            project_id: "pack".into(),
            name: "1.0".into(),
            version_number: "1.0".into(),
            version_type: "release".into(),
            downloads: 0,
            date_published: None,
            game_versions: vec!["1.20.1".into()],
            loaders: vec!["fabric".into()],
            minecraft_version: Some("1.20.1".into()),
            loader,
            file: file.then(|| PackFile {
                url: "https://cdn.modrinth.com/a.mrpack".into(),
                filename: "a.mrpack".into(),
                size: Some(10),
                hashes: FileHashes::default(),
            }),
            blocked,
            supported: loader.is_some() && file && !blocked,
        }
    }

    #[test]
    fn a_supported_version_becomes_the_pack_source() {
        let (pack, loader, minecraft) = resolve(
            PackProvider::Modrinth,
            "pack",
            version(Some(LoaderType::Fabric), true, false),
        )
        .unwrap();

        assert_eq!(pack.project_id, "pack");
        assert_eq!(pack.version_id, "v1");
        assert_eq!(pack.file_name, "a.mrpack");
        assert_eq!(loader, LoaderType::Fabric);
        assert_eq!(minecraft, "1.20.1");
    }

    #[test]
    fn an_unsupported_version_explains_why_through_the_catalog_reason() {
        let error = resolve(
            PackProvider::Modrinth,
            "pack",
            version(Some(LoaderType::Fabric), true, true),
        )
        .unwrap_err();

        assert_eq!(error.code, ErrorCode::Unsupported);
        assert!(error.text.mentions("catalog.unsupported.blocked"));
    }
}
