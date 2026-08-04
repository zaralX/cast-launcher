use crate::install::progress::Phase;
use crate::instance::{LoaderType, LocalPackKind, PackProvider};

const VANILLA: &[Phase] = &[
    Phase::new("java", "Java", 8),
    Phase::new("client", "Клиент", 12),
    Phase::new("libraries", "Библиотеки", 20),
    Phase::new("assets", "Ресурсы", 60),
];

const FABRIC: &[Phase] = &[
    Phase::new("java", "Java", 8),
    Phase::new("client", "Клиент", 12),
    Phase::new("libraries", "Библиотеки", 20),
    Phase::new("assets", "Ресурсы", 50),
    Phase::new("fabric", "Fabric", 10),
];

const FORGE: &[Phase] = &[
    Phase::new("java", "Java", 8),
    Phase::new("client", "Клиент", 10),
    Phase::new("libraries", "Библиотеки", 18),
    Phase::new("assets", "Ресурсы", 40),
    Phase::new("forge-installer", "Установщик Forge", 4),
    Phase::new("forge-libraries", "Библиотеки Forge", 12),
    Phase::new("forge-patch", "Сборка Forge", 8),
];

const NEOFORGE: &[Phase] = &[
    Phase::new("java", "Java", 8),
    Phase::new("client", "Клиент", 10),
    Phase::new("libraries", "Библиотеки", 18),
    Phase::new("assets", "Ресурсы", 40),
    Phase::new("neoforge-installer", "Установщик NeoForge", 4),
    Phase::new("neoforge-libraries", "Библиотеки NeoForge", 12),
    Phase::new("neoforge-patch", "Сборка NeoForge", 8),
];

const MODPACK: Phase = Phase::new("modpack", "Файлы модпака", 25);

const MODPACK_RESOLVE: Phase = Phase::new("modpack-resolve", "Список файлов пака", 5);

const CASTPACK_MANIFEST: Phase = Phase::new("castpack-manifest", "Манифест сборки", 4);

const CASTPACK_MODS: Phase = Phase::new("castpack-mods", "Список модов сборки", 4);

const CASTPACK: Phase = Phase::new("castpack", "Файлы сборки", 22);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Plain,
    Pack(PackProvider),
    LocalPack(LocalPackKind),
    CastPack(Option<PackProvider>),
}

impl Source {
    pub fn of(instance: &crate::instance::Instance) -> Self {
        match (&instance.castpack, &instance.pack, &instance.local_pack) {
            (Some(_), pack, _) => Self::CastPack(pack.as_ref().map(|pack| pack.provider)),
            (None, Some(pack), _) => Self::Pack(pack.provider),
            (None, None, Some(local)) => Self::LocalPack(local.kind),
            (None, None, None) => Self::Plain,
        }
    }
}

pub fn for_loader(loader: LoaderType) -> Vec<Phase> {
    match loader {
        LoaderType::Vanilla => VANILLA.to_vec(),
        LoaderType::Fabric => FABRIC.to_vec(),
        LoaderType::Forge => FORGE.to_vec(),
        LoaderType::NeoForge => NEOFORGE.to_vec(),
    }
}

fn modpack_phases(provider: PackProvider) -> Vec<Phase> {
    match provider {
        PackProvider::Modrinth => vec![MODPACK],
        PackProvider::CurseForge => vec![MODPACK_RESOLVE, MODPACK],
    }
}

fn extra_phases(source: Source) -> Vec<Phase> {
    match source {
        Source::Plain => Vec::new(),
        Source::Pack(provider) => modpack_phases(provider),
        Source::LocalPack(kind) => match kind.resolves_files() {
            true => vec![MODPACK_RESOLVE, MODPACK],
            false => vec![MODPACK],
        },
        Source::CastPack(base) => {
            let mut phases = vec![CASTPACK_MANIFEST];

            if base == Some(PackProvider::CurseForge) {
                phases.push(MODPACK_RESOLVE);
            }

            phases.push(CASTPACK_MODS);
            phases.push(CASTPACK);
            phases
        }
    }
}

pub fn for_install(loader: LoaderType, source: Source) -> Vec<Phase> {
    let base = for_loader(loader);
    let extra = extra_phases(source);

    if extra.is_empty() {
        return base;
    }

    let weight: u32 = extra.iter().map(|phase| phase.weight).sum();

    let mut phases = rescale(&base, 100 - weight);
    phases.extend(extra);
    phases
}

fn rescale(phases: &[Phase], target: u32) -> Vec<Phase> {
    let total: u32 = phases.iter().map(|phase| phase.weight).sum();

    if total == 0 {
        return phases.to_vec();
    }

    let mut used = 0;

    phases
        .iter()
        .enumerate()
        .map(|(index, phase)| {
            let weight = if index + 1 == phases.len() {
                target.saturating_sub(used)
            } else {
                phase.weight * target / total
            };

            used += weight;

            Phase::new(phase.key, phase.label, weight)
        })
        .collect()
}

pub fn job_id(instance_id: &str, phase: &str) -> String {
    format!("{}{phase}", job_prefix(instance_id))
}

pub fn job_prefix(instance_id: &str) -> String {
    format!("install:{instance_id}:")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_loader_has_a_full_scale() {
        for loader in LoaderType::ALL {
            let total: u32 = for_loader(loader).iter().map(|phase| phase.weight).sum();
            assert_eq!(total, 100, "фазы {loader:?} должны в сумме давать 100");
        }
    }

    fn sources() -> Vec<Source> {
        let mut sources = vec![Source::Plain, Source::CastPack(None)];

        for provider in PackProvider::ALL {
            sources.push(Source::Pack(provider));
            sources.push(Source::CastPack(Some(provider)));
        }

        for kind in LocalPackKind::ALL {
            sources.push(Source::LocalPack(kind));
        }

        sources
    }

    #[test]
    fn every_kind_of_install_still_adds_up_to_a_full_scale() {
        for source in sources() {
            for loader in LoaderType::ALL {
                let phases = for_install(loader, source);
                let total: u32 = phases.iter().map(|phase| phase.weight).sum();

                assert_eq!(
                    total, 100,
                    "фазы {loader:?} из источника {source:?} должны в сумме давать 100"
                );
            }
        }
    }

    #[test]
    fn the_last_phase_is_always_the_one_that_lays_out_the_files() {
        for provider in PackProvider::ALL {
            assert_eq!(for_install(LoaderType::Forge, Source::Pack(provider)).last().unwrap().key, "modpack");
            assert_eq!(
                for_install(LoaderType::Forge, Source::CastPack(Some(provider)))
                    .last()
                    .unwrap()
                    .key,
                "castpack"
            );
        }

        assert_eq!(for_install(LoaderType::Forge, Source::CastPack(None)).last().unwrap().key, "castpack");
    }

    #[test]
    fn curseforge_gets_an_extra_phase_for_resolving_file_links() {
        let loader = LoaderType::Fabric;

        let modrinth = for_install(loader, Source::Pack(PackProvider::Modrinth));
        let curseforge = for_install(loader, Source::Pack(PackProvider::CurseForge));

        assert_eq!(modrinth.len(), for_loader(loader).len() + 1);
        assert_eq!(curseforge.len(), for_loader(loader).len() + 2);

        assert!(!modrinth.iter().any(|phase| phase.key == "modpack-resolve"));

        let keys: Vec<_> = curseforge.iter().map(|phase| phase.key).collect();
        let at = |key: &str| keys.iter().position(|item| *item == key).unwrap();

        assert!(at("modpack-resolve") < at("modpack"), "сначала список, потом загрузка");
    }

    #[test]
    fn a_castpack_reads_its_manifest_before_anything_else_it_owns() {
        let phases = for_install(LoaderType::Forge, Source::CastPack(Some(PackProvider::CurseForge)));
        let keys: Vec<_> = phases.iter().map(|phase| phase.key).collect();
        let at = |key: &str| keys.iter().position(|item| *item == key).unwrap();

        assert!(at("castpack-manifest") < at("modpack-resolve"));
        assert!(at("modpack-resolve") < at("castpack-mods"));
        assert!(at("castpack-mods") < at("castpack"));
    }

    #[test]
    fn a_castpack_without_a_base_pack_does_not_resolve_pack_files() {
        let phases = for_install(LoaderType::Fabric, Source::CastPack(None));
        let keys: Vec<_> = phases.iter().map(|phase| phase.key).collect();

        assert!(!keys.contains(&"modpack-resolve"));
        assert!(!keys.contains(&"modpack"));
        assert!(keys.contains(&"castpack-manifest"));
    }

    #[test]
    fn without_a_pack_the_phases_are_untouched() {
        let plain = for_install(LoaderType::Fabric, Source::Plain);
        let base = for_loader(LoaderType::Fabric);

        assert_eq!(plain.len(), base.len());
        assert!(plain.iter().zip(&base).all(|(a, b)| a.key == b.key && a.weight == b.weight));
    }

    #[test]
    fn phase_keys_stay_unique_for_every_source() {
        for source in sources() {
            let phases = for_install(LoaderType::Forge, source);
            let mut keys: Vec<_> = phases.iter().map(|phase| phase.key).collect();
            keys.sort_unstable();
            keys.dedup();

            assert_eq!(keys.len(), phases.len(), "дубли ключей у {source:?}");
        }
    }

    #[test]
    fn a_pack_from_a_file_lays_out_its_files_last_and_looks_up_links_only_for_curseforge() {
        let loader = LoaderType::Fabric;

        for kind in LocalPackKind::ALL {
            let phases = for_install(loader, Source::LocalPack(kind));
            let keys: Vec<_> = phases.iter().map(|phase| phase.key).collect();

            assert_eq!(*keys.last().unwrap(), "modpack", "{kind:?}");
            assert_eq!(
                keys.contains(&"modpack-resolve"),
                kind == LocalPackKind::CurseForge,
                "ссылки на файлы ищем только там, где их нет в архиве: {kind:?}"
            );
        }
    }

    #[test]
    fn an_instance_imported_from_a_file_is_installed_as_a_modpack() {
        use crate::instance::Instance;
        use serde_json::json;

        let local: Instance = serde_json::from_value(json!({
            "id": "a", "name": "a", "minecraftVersion": "1.20.1", "type": "forge",
            "localPack": {"kind": "multimc", "name": "TerraFirmaGreg", "version": ""}
        }))
        .unwrap();

        assert_eq!(Source::of(&local), Source::LocalPack(LocalPackKind::MultiMc));
    }

    #[test]
    fn a_castpack_instance_is_recognised_even_when_it_stands_on_a_modpack() {
        use crate::instance::Instance;
        use serde_json::json;

        let plain: Instance = serde_json::from_value(json!({
            "id": "a", "name": "a", "minecraftVersion": "1.20.1", "type": "fabric"
        }))
        .unwrap();
        assert_eq!(Source::of(&plain), Source::Plain);

        let with_pack: Instance = serde_json::from_value(json!({
            "id": "a", "name": "a", "minecraftVersion": "1.20.1", "type": "fabric",
            "pack": {"provider": "modrinth", "projectId": "p", "versionId": "v", "fileUrl": "https://x"}
        }))
        .unwrap();
        assert_eq!(Source::of(&with_pack), Source::Pack(PackProvider::Modrinth));

        let castpack: Instance = serde_json::from_value(json!({
            "id": "a", "name": "a", "minecraftVersion": "1.20.1", "type": "fabric",
            "castpack": {"catalogId": "rpg", "manifestUrl": "https://x/m.json"},
            "pack": {"provider": "curseforge", "projectId": "p", "versionId": "v", "fileUrl": "https://x"}
        }))
        .unwrap();
        assert_eq!(Source::of(&castpack), Source::CastPack(Some(PackProvider::CurseForge)));
    }

    #[test]
    fn every_loader_downloads_the_vanilla_client() {
        for loader in LoaderType::ALL {
            assert!(for_loader(loader).iter().any(|phase| phase.key == "client"), "{loader:?}");
        }
    }

    #[test]
    fn an_installer_driven_loader_builds_the_client_after_downloading_its_libraries() {
        for (loader, prefix) in [(LoaderType::Forge, "forge"), (LoaderType::NeoForge, "neoforge")] {
            let phases = for_loader(loader);
            let keys: Vec<_> = phases.iter().map(|phase| phase.key).collect();
            let at = |suffix: &str| {
                keys.iter()
                    .position(|key| *key == format!("{prefix}-{suffix}"))
                    .unwrap_or_else(|| panic!("у {loader:?} нет фазы {prefix}-{suffix}"))
            };

            assert!(at("installer") < at("libraries"));
            assert!(at("libraries") < at("patch"));
        }
    }

    #[test]
    fn loaders_installed_the_same_way_still_report_their_own_phases() {
        let forge: Vec<_> = for_loader(LoaderType::Forge).iter().map(|phase| phase.key).collect();
        let neoforge: Vec<_> = for_loader(LoaderType::NeoForge).iter().map(|phase| phase.key).collect();

        assert!(forge.iter().all(|key| !neoforge.contains(key) || !key.contains('-')));
    }

    #[test]
    fn phase_keys_are_unique_within_a_loader() {
        for loader in LoaderType::ALL {
            let phases = for_loader(loader);
            let mut keys: Vec<_> = phases.iter().map(|phase| phase.key).collect();
            keys.sort_unstable();
            keys.dedup();

            assert_eq!(keys.len(), phases.len(), "дубли ключей у {loader:?}");
        }
    }

    #[test]
    fn job_ids_share_the_instance_prefix() {
        assert_eq!(job_id("abc", "assets"), "install:abc:assets");
        assert!(job_id("abc", "assets").starts_with(&job_prefix("abc")));
        assert!(!job_id("abc", "assets").starts_with(&job_prefix("ab")));
    }
}
