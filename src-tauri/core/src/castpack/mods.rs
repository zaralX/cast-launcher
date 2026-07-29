use std::path::Path;

use crate::error::{CommandError, CommandResult};
use crate::fs_util::{relative_key, safe_join};
use crate::instance::PackProvider;
use crate::net::download::DownloadTask;
use crate::packs::BlockedFile;

use super::manifest::ModRef;

#[derive(Debug, Default)]
pub struct ResolvedMods {
    pub files: Vec<(String, DownloadTask)>,
    pub blocked: Vec<BlockedFile>,
}

pub async fn resolve(refs: &[ModRef<'_>], minecraft_dir: &Path) -> CommandResult<ResolvedMods> {
    let mut resolved = ResolvedMods::default();

    let modrinth: Vec<&ModRef<'_>> = refs.iter().filter(|entry| provider_of(entry) == Some(PackProvider::Modrinth)).collect();
    let curseforge: Vec<&ModRef<'_>> = refs.iter().filter(|entry| provider_of(entry) == Some(PackProvider::CurseForge)).collect();

    if !modrinth.is_empty() {
        resolved.files.extend(from_modrinth(&modrinth, minecraft_dir).await?);
    }

    if !curseforge.is_empty() {
        let mut from_curseforge = from_curseforge(&curseforge, minecraft_dir).await?;

        resolved.files.append(&mut from_curseforge.files);
        resolved.blocked.append(&mut from_curseforge.blocked);
    }

    Ok(resolved)
}

fn provider_of(entry: &ModRef<'_>) -> Option<PackProvider> {
    match entry {
        ModRef::Catalog { provider, .. } => Some(*provider),
        ModRef::Direct { .. } => None,
    }
}

async fn from_modrinth(
    refs: &[&ModRef<'_>],
    minecraft_dir: &Path,
) -> CommandResult<Vec<(String, DownloadTask)>> {
    let version_ids: Vec<String> = refs
        .iter()
        .filter_map(|entry| match entry {
            ModRef::Catalog { version_id, .. } => Some((*version_id).to_string()),
            ModRef::Direct { .. } => None,
        })
        .collect();

    let files = crate::modrinth::catalog_files(&version_ids).await?;
    let mut resolved = Vec::new();

    for entry in refs {
        let ModRef::Catalog { version_id, optional, .. } = entry else { continue };

        let file = files.get(*version_id).ok_or_else(|| {
            CommandError::manifest(format!("Modrinth не отдал файл версии {version_id}"))
        })?;

        let key = target_key(file.folder, &file.file_name, *optional)?;

        resolved.push((
            key.clone(),
            DownloadTask::verified(
                file.url.clone(),
                safe_join(minecraft_dir, &key)?,
                file.size,
                file.sha1.clone(),
            ),
        ));
    }

    Ok(resolved)
}

async fn from_curseforge(refs: &[&ModRef<'_>], minecraft_dir: &Path) -> CommandResult<ResolvedMods> {
    use crate::curseforge::pack::ManifestFile;

    let mut entries = Vec::with_capacity(refs.len());

    for entry in refs {
        let ModRef::Catalog { project_id, version_id, optional, .. } = entry else { continue };

        entries.push(ManifestFile {
            project_id: numeric(project_id, "projectId")?,
            file_id: numeric(version_id, "versionId")?,
            required: !optional,
        });
    }

    let (files, blocked) = crate::curseforge::pack::resolve_entries(&entries, minecraft_dir).await?;

    Ok(ResolvedMods { files, blocked })
}

fn target_key(folder: &str, file_name: &str, optional: bool) -> CommandResult<String> {
    let name = crate::curseforge::pack::sanitize(file_name);

    let key = relative_key(&format!("{folder}/{name}"))?;

    Ok(match optional {
        true => format!("{key}.disabled"),
        false => key,
    })
}

fn numeric(value: &str, field: &str) -> CommandResult<u64> {
    value.trim().parse().map_err(|_| {
        CommandError::manifest(format!(
            "У мода CurseForge поле {field} должно быть числом, а не «{value}»"
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_mod_lands_in_the_folder_its_catalog_says() {
        assert_eq!(target_key("mods", "jei-1.0.jar", false).unwrap(), "mods/jei-1.0.jar");
        assert_eq!(
            target_key("shaderpacks", "BSL.zip", false).unwrap(),
            "shaderpacks/BSL.zip"
        );
    }

    #[test]
    fn an_optional_mod_arrives_switched_off() {
        assert_eq!(
            target_key("mods", "shaders.jar", true).unwrap(),
            "mods/shaders.jar.disabled"
        );
    }

    #[test]
    fn a_file_name_from_the_catalog_cannot_walk_out_of_the_folder() {
        let key = target_key("mods", "../../evil.jar", false).unwrap();

        assert!(key.starts_with("mods/"), "{key}");
        assert_eq!(key.split('/').count(), 2, "разделители в имени файла обезврежены: {key}");
    }

    #[test]
    fn curseforge_identifiers_have_to_be_numbers() {
        assert_eq!(numeric(" 238222 ", "projectId").unwrap(), 238222);

        let error = numeric("AANobbMI", "projectId").unwrap_err();
        assert!(error.message.contains("projectId"), "{}", error.message);
    }

    #[test]
    fn references_are_split_by_catalog() {
        let modrinth = ModRef::Catalog {
            provider: PackProvider::Modrinth,
            project_id: "AANobbMI",
            version_id: "xyz",
            optional: false,
        };
        let curseforge = ModRef::Catalog {
            provider: PackProvider::CurseForge,
            project_id: "238222",
            version_id: "5432101",
            optional: false,
        };
        let direct = ModRef::Direct {
            url: "https://cdn.zaralx.ru/core.jar",
            key: "mods/core.jar".into(),
            sha1: "aaa",
            size: None,
        };

        assert_eq!(provider_of(&modrinth), Some(PackProvider::Modrinth));
        assert_eq!(provider_of(&curseforge), Some(PackProvider::CurseForge));
        assert_eq!(provider_of(&direct), None, "прямые ссылки в каталог не ходят");
    }

    #[tokio::test]
    async fn an_empty_list_does_not_touch_the_network() {
        let resolved = resolve(&[], Path::new("/mc")).await.unwrap();

        assert!(resolved.files.is_empty());
        assert!(resolved.blocked.is_empty());
    }
}
