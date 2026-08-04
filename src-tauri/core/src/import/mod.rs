pub mod copy;
pub mod ini;
pub mod modrinth;
pub mod prism;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use copy::{CopyStats, Progress};

use crate::error::{CommandError, CommandResult};
use crate::instance::{Instance, InstanceSettings, LoaderType, Playtime};
use crate::paths::LauncherPaths;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LauncherKind {
    Prism,
    Modrinth,
}

impl LauncherKind {
    pub const ALL: [Self; 2] = [Self::Prism, Self::Modrinth];

    pub fn label(self) -> &'static str {
        match self {
            Self::Prism => "PrismLauncher",
            Self::Modrinth => "Modrinth App",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Self::Prism => "prism",
            Self::Modrinth => "modrinth",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedPack {
    pub provider: String,
    pub project_id: String,
    pub version_id: String,
    pub version_name: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScannedInstance {
    pub folder: String,
    pub name: String,
    pub description: String,
    pub minecraft_version: String,
    pub loader: Option<LoaderType>,
    pub loader_version: Option<String>,
    pub loader_label: String,
    pub icon: Option<String>,
    #[serde(skip)]
    pub icon_source: Option<PathBuf>,
    pub settings: InstanceSettings,
    pub playtime: Playtime,
    pub pack: Option<ManagedPack>,
    pub blocked: Option<String>,
}

impl ScannedInstance {
    pub fn new(folder: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            folder: folder.into(),
            name: name.into(),
            description: String::new(),
            minecraft_version: String::new(),
            loader: None,
            loader_version: None,
            loader_label: "Vanilla".into(),
            icon: None,
            icon_source: None,
            settings: InstanceSettings::default(),
            playtime: Playtime::default(),
            pack: None,
            blocked: None,
        }
    }

    pub fn is_importable(&self) -> bool {
        self.blocked.is_none()
    }

    pub fn to_instance(&self, id: String, icon: String) -> CommandResult<Instance> {
        if let Some(reason) = &self.blocked {
            return Err(CommandError::manifest(format!(
                "Сборку «{}» перенести нельзя: {reason}",
                self.name
            )));
        }

        let loader = self
            .loader
            .ok_or_else(|| CommandError::manifest(format!("У сборки «{}» нет загрузчика", self.name)))?;

        Ok(Instance {
            id,
            name: self.name.clone(),
            description: self.description.clone(),
            minecraft_version: self.minecraft_version.clone(),
            icon,
            loader,
            installed: false,
            version: 1,
            loader_version: self.loader_version.clone(),
            custom_id: None,
            pack: None,
            castpack: None,
            local_pack: None,
            settings: self.settings.clone(),
            playtime: self.playtime,
            dir: String::new(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct SharedTargets {
    pub libraries: PathBuf,
    pub asset_indexes: PathBuf,
    pub asset_objects: PathBuf,
    pub java_runtimes: PathBuf,
}

impl SharedTargets {
    pub fn of(paths: &LauncherPaths) -> Self {
        Self {
            libraries: paths.libraries(),
            asset_indexes: paths.asset_indexes(),
            asset_objects: paths.asset_objects(),
            java_runtimes: paths.java_runtimes(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct InstanceTargets {
    pub minecraft: PathBuf,
    pub client_jar: PathBuf,
    pub loader_installer: Option<PathBuf>,
}

#[derive(Debug)]
pub enum Source {
    Prism(PathBuf),
    Modrinth(modrinth::Root),
}

impl Source {
    pub async fn open(kind: LauncherKind, path: &str) -> CommandResult<Self> {
        let path = path.trim();

        if path.is_empty() {
            return Err(CommandError::fs(format!("Не указан каталог {}", kind.label())));
        }

        match kind {
            LauncherKind::Prism => Ok(Self::Prism(prism::open(Path::new(path))?)),
            LauncherKind::Modrinth => Ok(Self::Modrinth(modrinth::open(Path::new(path)).await?)),
        }
    }

    pub fn kind(&self) -> LauncherKind {
        match self {
            Self::Prism(_) => LauncherKind::Prism,
            Self::Modrinth(_) => LauncherKind::Modrinth,
        }
    }

    pub async fn scan(&self) -> CommandResult<Vec<ScannedInstance>> {
        let mut found = match self {
            Self::Prism(root) => prism::scan(root).await?,
            Self::Modrinth(root) => modrinth::scan(root).await,
        };

        found.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

        Ok(found)
    }

    pub async fn copy_shared(
        &self,
        options: &ImportOptions,
        targets: &SharedTargets,
        progress: &Progress<'_>,
        on_step: impl Fn(&str),
    ) -> CommandResult<()> {
        match self {
            Self::Prism(root) => prism::copy_shared(root, options, targets, progress, on_step).await,
            Self::Modrinth(root) => modrinth::copy_shared(root, options, targets, progress, on_step).await,
        }
    }

    pub async fn copy_instance(
        &self,
        scanned: &ScannedInstance,
        targets: &InstanceTargets,
        progress: &Progress<'_>,
    ) -> CommandResult<()> {
        match self {
            Self::Prism(root) => prism::copy_instance(root, scanned, targets, progress).await,
            Self::Modrinth(root) => modrinth::copy_instance(root, scanned, targets, progress).await,
        }
    }

    pub fn loader_installer_target(
        &self,
        paths: &LauncherPaths,
        instance: &Instance,
    ) -> Option<PathBuf> {
        match self {
            Self::Prism(_) => prism::loader_installer_target(paths, instance),
            Self::Modrinth(_) => None,
        }
    }
}

pub fn detect(kind: LauncherKind) -> Option<PathBuf> {
    match kind {
        LauncherKind::Prism => prism::detect(),
        LauncherKind::Modrinth => modrinth::detect(),
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
    fn a_freshly_scanned_instance_is_vanilla_shaped_and_unblocked() {
        let scanned = ScannedInstance::new("папка", "Имя");

        assert_eq!(scanned.folder, "папка");
        assert_eq!(scanned.loader_label, "Vanilla");
        assert!(scanned.loader.is_none());
        assert!(scanned.is_importable());
        assert!(scanned.icon.is_none() && scanned.icon_source.is_none());
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
        assert_eq!(
            serde_json::to_value(LauncherKind::Modrinth).unwrap(),
            serde_json::json!("modrinth")
        );
    }

    #[tokio::test]
    async fn an_empty_path_is_refused_for_every_launcher() {
        for kind in LauncherKind::ALL {
            let error = Source::open(kind, "   ").await.unwrap_err();

            assert!(error.message.contains(kind.label()), "{kind:?}");
        }
    }

    #[tokio::test]
    async fn a_folder_belonging_to_no_launcher_is_refused_with_a_hint() {
        let dir = std::env::temp_dir().join(format!("cast-import-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();

        let path = dir.display().to_string();

        assert!(Source::open(LauncherKind::Prism, &path)
            .await
            .unwrap_err()
            .message
            .contains("instances"));

        assert!(Source::open(LauncherKind::Modrinth, &path)
            .await
            .unwrap_err()
            .message
            .contains("app.db"));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn instances_arrive_sorted_by_name_whatever_the_launcher() {
        let root = std::env::temp_dir().join(format!("cast-import-{}", uuid::Uuid::new_v4()));

        for (folder, name) in [("c", "Ягоды"), ("a", "яблоки"), ("b", "Абрикос")] {
            let dir = root.join(prism::INSTANCES).join(folder);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join(prism::CONFIG_FILE), format!("[General]\nname={name}")).unwrap();
            std::fs::write(dir.join(prism::PACK_FILE), FABRIC_PACK).unwrap();
        }

        let source = Source::open(LauncherKind::Prism, &root.display().to_string())
            .await
            .unwrap();

        let names: Vec<_> = source
            .scan()
            .await
            .unwrap()
            .into_iter()
            .map(|instance| instance.name)
            .collect();

        assert_eq!(names, vec!["Абрикос", "яблоки", "Ягоды"]);
        assert_eq!(source.kind(), LauncherKind::Prism);

        std::fs::remove_dir_all(&root).ok();
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
