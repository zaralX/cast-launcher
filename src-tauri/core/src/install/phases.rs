use crate::install::progress::Phase;
use crate::instance::LoaderType;

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
    Phase::new("libraries", "Библиотеки", 20),
    Phase::new("assets", "Ресурсы", 45),
    Phase::new("forge-installer", "Установщик Forge", 12),
    Phase::new("forge-install", "Установка Forge", 15),
];

const MODPACK: Phase = Phase::new("modpack", "Файлы модпака", 25);

pub fn for_loader(loader: LoaderType) -> Vec<Phase> {
    match loader {
        LoaderType::Vanilla => VANILLA.to_vec(),
        LoaderType::Fabric => FABRIC.to_vec(),
        LoaderType::Forge => FORGE.to_vec(),
    }
}

pub fn for_install(loader: LoaderType, with_modpack: bool) -> Vec<Phase> {
    let base = for_loader(loader);

    if !with_modpack {
        return base;
    }

    let mut phases = rescale(&base, 100 - MODPACK.weight);
    phases.push(MODPACK);
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
        for loader in [LoaderType::Vanilla, LoaderType::Fabric, LoaderType::Forge] {
            let total: u32 = for_loader(loader).iter().map(|phase| phase.weight).sum();
            assert_eq!(total, 100, "фазы {loader:?} должны в сумме давать 100");
        }
    }

    #[test]
    fn a_modpack_install_still_adds_up_to_a_full_scale() {
        for loader in [LoaderType::Vanilla, LoaderType::Fabric, LoaderType::Forge] {
            let phases = for_install(loader, true);
            let total: u32 = phases.iter().map(|phase| phase.weight).sum();

            assert_eq!(total, 100, "фазы {loader:?} с модпаком должны в сумме давать 100");
            assert_eq!(phases.last().unwrap().key, "modpack");
            assert_eq!(phases.len(), for_loader(loader).len() + 1);
        }
    }

    #[test]
    fn without_a_modpack_the_phases_are_untouched() {
        let plain = for_install(LoaderType::Fabric, false);
        let base = for_loader(LoaderType::Fabric);

        assert_eq!(plain.len(), base.len());
        assert!(plain.iter().zip(&base).all(|(a, b)| a.key == b.key && a.weight == b.weight));
    }

    #[test]
    fn forge_has_no_client_phase() {
        assert!(!for_loader(LoaderType::Forge).iter().any(|phase| phase.key == "client"));
        assert!(for_loader(LoaderType::Fabric).iter().any(|phase| phase.key == "client"));
    }

    #[test]
    fn phase_keys_are_unique_within_a_loader() {
        for loader in [LoaderType::Vanilla, LoaderType::Fabric, LoaderType::Forge] {
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
