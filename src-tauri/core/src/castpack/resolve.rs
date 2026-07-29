use std::collections::{BTreeMap, BTreeSet};

use crate::error::{CommandError, CommandResult};
use crate::instance::LoaderType;
use crate::net::download::DownloadTask;
use crate::packs::{BlockedFile, ResolvedPack};

use super::manifest::SeedFile;

#[derive(Debug, Default)]
pub struct Overlay {
    pub minecraft_version: Option<String>,
    pub loader: Option<(LoaderType, Option<String>)>,
    pub files: Vec<(String, DownloadTask)>,
    pub seed: Vec<SeedFile>,
    pub delete: BTreeSet<String>,
    pub blocked: Vec<BlockedFile>,
    pub recommended_ram: Option<u32>,
}

pub fn merge(base: Option<ResolvedPack>, overlay: Overlay) -> CommandResult<ResolvedPack> {
    let mut owned: BTreeMap<String, DownloadTask> = BTreeMap::new();
    let mut overrides = Vec::new();
    let mut blocked: Vec<BlockedFile> = Vec::new();
    let mut minecraft_version = None;
    let mut loader = None;
    let mut recommended_ram = None;

    if let Some(base) = base {
        if base.paths.len() != base.tasks.len() {
            return Err(CommandError::manifest(
                "Базовый модпак вернул рассогласованный список файлов",
            ));
        }

        owned.extend(base.paths.into_iter().zip(base.tasks));
        overrides = base.overrides;
        blocked = base.blocked;
        minecraft_version = Some(base.minecraft_version);
        loader = Some((base.loader, base.loader_version));
        recommended_ram = base.recommended_ram;
    }

    let mut added: BTreeSet<String> = BTreeSet::new();

    for (key, task) in overlay.files {
        if !added.insert(key.clone()) {
            return Err(CommandError::manifest(format!(
                "В сборке дважды указан один и тот же файл: {key}"
            )));
        }

        owned.insert(key, task);
    }

    for key in &overlay.delete {
        owned.remove(key);
    }

    blocked.retain(|file| !added.contains(&file.target_path) && !overlay.delete.contains(&file.target_path));
    blocked.extend(overlay.blocked);

    let seed = overlay
        .seed
        .into_iter()
        .filter(|file| !owned.contains_key(&file.key) && !overlay.delete.contains(&file.key))
        .collect();

    if let Some(version) = overlay.minecraft_version {
        minecraft_version = Some(version);
    }

    if let Some(spec) = overlay.loader {
        loader = Some(spec);
    }

    if overlay.recommended_ram.is_some() {
        recommended_ram = overlay.recommended_ram;
    }

    let minecraft_version = minecraft_version.filter(|version| !version.is_empty()).ok_or_else(|| {
        CommandError::manifest("У сборки не удалось определить версию Minecraft")
    })?;

    let (loader, loader_version) = loader.unwrap_or((LoaderType::Vanilla, None));

    let (paths, tasks): (Vec<String>, Vec<DownloadTask>) = owned.into_iter().unzip();

    Ok(ResolvedPack {
        minecraft_version,
        loader,
        loader_version,
        tasks,
        paths,
        overrides,
        blocked,
        recommended_ram,
        seed,
        delete: overlay.delete.into_iter().collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn task(url: &str) -> DownloadTask {
        DownloadTask::verified(url.to_string(), PathBuf::from("/mc").join(url), None, Some("a".into()))
    }

    fn base() -> ResolvedPack {
        ResolvedPack {
            minecraft_version: "1.20.1".into(),
            loader: LoaderType::Forge,
            loader_version: Some("1.20.1-47.2.0".into()),
            tasks: vec![task("jei.jar"), task("optifine.jar")],
            paths: vec!["mods/jei.jar".into(), "mods/optifine.jar".into()],
            overrides: vec!["overrides".into()],
            blocked: Vec::new(),
            recommended_ram: Some(4096),
            seed: Vec::new(),
            delete: Vec::new(),
        }
    }

    fn overlay() -> Overlay {
        Overlay::default()
    }

    fn seed(key: &str) -> SeedFile {
        SeedFile {
            key: key.to_string(),
            task: task(key),
        }
    }

    #[test]
    fn without_a_base_the_overlay_is_the_whole_pack() {
        let merged = merge(
            None,
            Overlay {
                minecraft_version: Some("1.21.1".into()),
                loader: Some((LoaderType::Fabric, Some("0.16.0".into()))),
                files: vec![("mods/sodium.jar".into(), task("sodium.jar"))],
                ..overlay()
            },
        )
        .unwrap();

        assert_eq!(merged.minecraft_version, "1.21.1");
        assert_eq!(merged.loader, LoaderType::Fabric);
        assert_eq!(merged.paths, vec!["mods/sodium.jar"]);
        assert!(merged.overrides.is_empty(), "распаковывать нечего");
    }

    #[test]
    fn a_pack_without_a_minecraft_version_anywhere_is_an_error() {
        assert!(merge(None, overlay()).is_err());
    }

    #[test]
    fn the_base_pack_decides_the_loader_until_the_manifest_overrides_it() {
        let kept = merge(Some(base()), overlay()).unwrap();

        assert_eq!(kept.loader, LoaderType::Forge);
        assert_eq!(kept.loader_version.as_deref(), Some("1.20.1-47.2.0"));
        assert_eq!(kept.minecraft_version, "1.20.1");

        let overridden = merge(
            Some(base()),
            Overlay {
                minecraft_version: Some("1.21.1".into()),
                loader: Some((LoaderType::NeoForge, Some("21.1.243".into()))),
                ..overlay()
            },
        )
        .unwrap();

        assert_eq!(overridden.loader, LoaderType::NeoForge);
        assert_eq!(overridden.minecraft_version, "1.21.1");
    }

    #[test]
    fn a_mod_from_the_manifest_replaces_the_one_from_the_pack() {
        let merged = merge(
            Some(base()),
            Overlay {
                files: vec![("mods/jei.jar".into(), task("jei-new.jar"))],
                ..overlay()
            },
        )
        .unwrap();

        assert_eq!(merged.paths.len(), 2, "путь тот же, файл другой");

        let at = merged.paths.iter().position(|path| path == "mods/jei.jar").unwrap();
        assert_eq!(merged.tasks[at].url, "jei-new.jar");
    }

    #[test]
    fn delete_takes_files_away_from_the_base_pack() {
        let merged = merge(
            Some(base()),
            Overlay {
                delete: BTreeSet::from(["mods/optifine.jar".to_string()]),
                ..overlay()
            },
        )
        .unwrap();

        assert_eq!(merged.paths, vec!["mods/jei.jar"]);
        assert_eq!(merged.delete, vec!["mods/optifine.jar"]);
    }

    #[test]
    fn a_file_the_manifest_adds_back_survives_its_own_delete_list() {
        let merged = merge(
            Some(base()),
            Overlay {
                files: vec![("mods/optifine.jar".into(), task("optifine-new.jar"))],
                delete: BTreeSet::from(["mods/optifine.jar".to_string()]),
                ..overlay()
            },
        )
        .unwrap();

        assert!(
            !merged.paths.contains(&"mods/optifine.jar".to_string()),
            "delete применяется последним и выигрывает - иначе непонятно, что имел в виду автор"
        );
    }

    #[test]
    fn seeded_files_step_aside_when_the_pack_owns_the_same_path() {
        let merged = merge(
            Some(base()),
            Overlay {
                files: vec![("options.txt".into(), task("options.txt"))],
                seed: vec![seed("options.txt"), seed("servers.dat")],
                ..overlay()
            },
        )
        .unwrap();

        let seeded: Vec<_> = merged.seed.iter().map(|file| file.key.as_str()).collect();
        assert_eq!(seeded, vec!["servers.dat"]);
    }

    #[test]
    fn the_same_path_twice_in_one_manifest_is_an_error() {
        let twice = merge(
            None,
            Overlay {
                minecraft_version: Some("1.20.1".into()),
                files: vec![
                    ("mods/a.jar".into(), task("a.jar")),
                    ("mods/a.jar".into(), task("a-again.jar")),
                ],
                ..overlay()
            },
        );

        assert!(twice.unwrap_err().message.contains("дважды"));
    }

    #[test]
    fn a_blocked_file_of_the_pack_disappears_once_the_manifest_brings_a_link() {
        let mut with_blocked = base();
        with_blocked.blocked = vec![BlockedFile {
            file_name: "entityculling.jar".into(),
            target_path: "mods/entityculling.jar".into(),
            website_url: "https://www.curseforge.com/x".into(),
            sha1: None,
            local_path: None,
        }];

        let merged = merge(
            Some(with_blocked),
            Overlay {
                files: vec![("mods/entityculling.jar".into(), task("entityculling.jar"))],
                ..overlay()
            },
        )
        .unwrap();

        assert!(merged.blocked.is_empty(), "руками качать больше нечего");
    }

    #[test]
    fn deleted_paths_do_not_stay_in_the_manual_download_queue() {
        let mut with_blocked = base();
        with_blocked.blocked = vec![BlockedFile {
            file_name: "entityculling.jar".into(),
            target_path: "mods/entityculling.jar".into(),
            website_url: "https://www.curseforge.com/x".into(),
            sha1: None,
            local_path: None,
        }];

        let merged = merge(
            Some(with_blocked),
            Overlay {
                delete: BTreeSet::from(["mods/entityculling.jar".to_string()]),
                ..overlay()
            },
        )
        .unwrap();

        assert!(merged.blocked.is_empty());
    }

    #[test]
    fn recommended_ram_comes_from_the_manifest_first() {
        assert_eq!(merge(Some(base()), overlay()).unwrap().recommended_ram, Some(4096));

        let overridden = merge(
            Some(base()),
            Overlay {
                recommended_ram: Some(8192),
                ..overlay()
            },
        )
        .unwrap();

        assert_eq!(overridden.recommended_ram, Some(8192));
    }

    #[test]
    fn paths_and_tasks_stay_side_by_side_after_a_merge() {
        let merged = merge(
            Some(base()),
            Overlay {
                files: vec![("config/rpg.toml".into(), task("rpg.toml"))],
                ..overlay()
            },
        )
        .unwrap();

        assert_eq!(merged.paths.len(), merged.tasks.len());

        for (path, task) in merged.paths.iter().zip(&merged.tasks) {
            let name = path.rsplit('/').next().unwrap();
            assert!(
                task.destination.ends_with(name),
                "{path} уехал не туда: {}",
                task.destination.display()
            );
        }
    }

    #[test]
    fn a_base_pack_with_mismatched_lists_is_refused() {
        let mut broken = base();
        broken.paths.pop();

        assert!(merge(Some(broken), overlay()).is_err());
    }
}
