pub mod copy;
pub mod ini;
pub mod prism;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use copy::CopyStats;
use prism::ScannedInstance;

use crate::error::{CommandError, CommandResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LauncherKind {
    Prism,
    Modrinth,
}

impl LauncherKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Prism => "PrismLauncher",
            Self::Modrinth => "Modrinth App",
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportOptions {
    pub assets: bool,
    pub libraries: bool,
    pub java: bool,
    pub icons: bool,
    pub link_packs: bool,
}

impl ImportOptions {
    pub fn copies_shared(&self) -> bool {
        self.assets || self.libraries || self.java
    }
}

impl Default for ImportOptions {
    fn default() -> Self {
        Self {
            assets: true,
            libraries: true,
            java: true,
            icons: true,
            link_packs: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ImportStage {
    Shared,
    Instances,
    Done,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportProgress {
    pub source: LauncherKind,
    pub stage: ImportStage,
    pub step: String,
    pub done: usize,
    pub total: usize,
    pub stats: CopyStats,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportedInstance {
    pub id: String,
    pub name: String,
    pub linked: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkippedInstance {
    pub name: String,
    pub reason: String,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportReport {
    pub imported: Vec<ImportedInstance>,
    pub skipped: Vec<SkippedInstance>,
    pub stats: CopyStats,
    pub cancelled: bool,
}

pub fn select(scanned: Vec<ScannedInstance>, folders: &[String]) -> (Vec<ScannedInstance>, ImportReport) {
    let mut report = ImportReport::default();
    let mut selected = Vec::new();

    for instance in scanned {
        if !folders.is_empty() && !folders.contains(&instance.folder) {
            continue;
        }

        match &instance.blocked {
            Some(reason) => report.skipped.push(SkippedInstance {
                name: instance.name.clone(),
                reason: reason.clone(),
            }),
            None => selected.push(instance),
        }
    }

    (selected, report)
}

#[derive(Default)]
pub struct ImportRegistry {
    running: AtomicBool,
    cancelled: AtomicBool,
}

impl ImportRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        if self.running.load(Ordering::SeqCst) {
            self.cancelled.store(true, Ordering::SeqCst);
        }
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    pub fn begin(self: &Arc<Self>) -> CommandResult<ImportGuard> {
        if self.running.swap(true, Ordering::SeqCst) {
            return Err(CommandError::fs("Перенос уже идёт, дождитесь его окончания"));
        }

        self.cancelled.store(false, Ordering::SeqCst);

        Ok(ImportGuard {
            registry: Arc::clone(self),
        })
    }
}

pub struct ImportGuard {
    registry: Arc<ImportRegistry>,
}

impl Drop for ImportGuard {
    fn drop(&mut self) {
        self.registry.cancelled.store(false, Ordering::SeqCst);
        self.registry.running.store(false, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FABRIC_PACK: &str = r#"{
        "components": [
            { "uid": "net.minecraft", "version": "1.21.11" },
            { "uid": "net.fabricmc.fabric-loader", "version": "0.18.4" }
        ]
    }"#;

    const QUILT_PACK: &str = r#"{
        "components": [
            { "uid": "net.minecraft", "version": "1.21.1" },
            { "uid": "org.quiltmc.quilt-loader", "version": "0.28.1" }
        ]
    }"#;

    fn scanned(folder: &str, pack: &str) -> ScannedInstance {
        prism::parse(folder, &format!("[General]\nname={folder}"), pack)
    }

    #[test]
    fn an_empty_selection_means_everything_we_can_take() {
        let (selected, report) = select(
            vec![scanned("a", FABRIC_PACK), scanned("b", FABRIC_PACK)],
            &[],
        );

        assert_eq!(selected.len(), 2);
        assert!(report.skipped.is_empty());
    }

    #[test]
    fn only_the_requested_folders_are_touched() {
        let (selected, report) = select(
            vec![scanned("a", FABRIC_PACK), scanned("b", FABRIC_PACK)],
            &["b".to_string()],
        );

        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].folder, "b");
        assert!(report.skipped.is_empty(), "невыбранное - не пропущенное");
    }

    #[test]
    fn unsupported_instances_end_up_in_the_report_with_a_reason() {
        let (selected, report) = select(
            vec![scanned("a", FABRIC_PACK), scanned("Beyond", QUILT_PACK)],
            &[],
        );

        assert_eq!(selected.len(), 1);
        assert_eq!(report.skipped.len(), 1);
        assert_eq!(report.skipped[0].name, "Beyond");
        assert!(report.skipped[0].reason.contains("Quilt"));
    }

    #[test]
    fn a_second_import_is_refused_while_the_first_one_runs() {
        let registry = Arc::new(ImportRegistry::new());

        let guard = registry.begin().expect("первый перенос стартует");
        assert!(registry.begin().is_err());

        drop(guard);
        assert!(registry.begin().is_ok(), "после завершения можно снова");
    }

    #[test]
    fn cancelling_only_counts_while_something_is_running() {
        let registry = Arc::new(ImportRegistry::new());

        registry.cancel();
        assert!(!registry.is_cancelled(), "отменять нечего");

        let _guard = registry.begin().unwrap();
        registry.cancel();
        assert!(registry.is_cancelled());
    }

    #[test]
    fn the_cancel_flag_does_not_survive_into_the_next_import() {
        let registry = Arc::new(ImportRegistry::new());

        let guard = registry.begin().unwrap();
        registry.cancel();
        drop(guard);

        let _next = registry.begin().unwrap();
        assert!(!registry.is_cancelled());
    }

    #[test]
    fn everything_is_carried_over_by_default() {
        let options = ImportOptions::default();

        assert!(options.assets && options.libraries && options.java && options.icons);
        assert!(options.link_packs);
        assert!(options.copies_shared());
    }

    #[test]
    fn shared_folders_are_skipped_when_nothing_from_them_is_wanted() {
        let options = ImportOptions {
            assets: false,
            libraries: false,
            java: false,
            icons: true,
            link_packs: true,
        };

        assert!(!options.copies_shared());
    }

    #[test]
    fn launcher_kinds_travel_as_lowercase() {
        assert_eq!(
            serde_json::to_value(LauncherKind::Prism).unwrap(),
            serde_json::json!("prism")
        );
    }

    #[test]
    fn progress_travels_as_camel_case() {
        let wire = serde_json::to_value(ImportProgress {
            source: LauncherKind::Prism,
            stage: ImportStage::Shared,
            step: "Библиотеки".into(),
            done: 1,
            total: 3,
            stats: CopyStats {
                files: 10,
                bytes: 2048,
                skipped: 4,
            },
        })
        .unwrap();

        assert_eq!(wire["stage"], "shared");
        assert_eq!(wire["source"], "prism");
        assert_eq!(wire["stats"]["files"], 10);
        assert_eq!(wire["stats"]["skipped"], 4);
    }

    #[test]
    fn an_empty_report_is_a_valid_one() {
        let wire = serde_json::to_value(ImportReport::default()).unwrap();

        assert_eq!(wire["imported"], serde_json::json!([]));
        assert_eq!(wire["cancelled"], false);
        assert_eq!(wire["stats"]["bytes"], 0);
    }
}
