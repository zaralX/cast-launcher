use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::config::{AppConfig, JavaMode};
use crate::error::{CommandError, CommandResult};
use crate::fs_util::{read_json_opt, write_json_atomic};
use crate::paths::LauncherPaths;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LoaderType {
    Vanilla,
    Fabric,
    Forge,
    NeoForge,
}

impl LoaderType {
    pub const ALL: [Self; 4] = [Self::Vanilla, Self::Fabric, Self::Forge, Self::NeoForge];

    pub fn label(self) -> &'static str {
        match self {
            Self::Vanilla => "Vanilla",
            Self::Fabric => "Fabric",
            Self::Forge => "Forge",
            Self::NeoForge => "NeoForge",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Self::Vanilla => "vanilla",
            Self::Fabric => "fabric",
            Self::Forge => "forge",
            Self::NeoForge => "neoforge",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PackProvider {
    Modrinth,
    CurseForge,
}

impl PackProvider {
    pub const ALL: [Self; 2] = [Self::Modrinth, Self::CurseForge];

    pub fn label(self) -> &'static str {
        match self {
            Self::Modrinth => "Modrinth",
            Self::CurseForge => "CurseForge",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Self::Modrinth => "modrinth",
            Self::CurseForge => "curseforge",
        }
    }

    pub fn archive_extension(self) -> &'static str {
        match self {
            Self::Modrinth => "mrpack",
            Self::CurseForge => "zip",
        }
    }

    pub fn from_key(kind: &str) -> Option<Self> {
        match kind.trim().to_ascii_lowercase().as_str() {
            "modrinth" => Some(Self::Modrinth),
            "flame" | "curseforge" => Some(Self::CurseForge),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LocalPackKind {
    Modrinth,
    CurseForge,
    MultiMc,
}

impl LocalPackKind {
    pub const ALL: [Self; 3] = [Self::Modrinth, Self::CurseForge, Self::MultiMc];

    pub fn label(self) -> &'static str {
        match self {
            Self::Modrinth => "Modrinth (.mrpack)",
            Self::CurseForge => "CurseForge",
            Self::MultiMc => "MultiMC / Prism",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Self::Modrinth => "modrinth",
            Self::CurseForge => "curseforge",
            Self::MultiMc => "multimc",
        }
    }
    pub fn resolves_files(self) -> bool {
        self == Self::CurseForge
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalPackSource {
    pub kind: LocalPackKind,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackSource {
    pub provider: PackProvider,
    pub project_id: String,
    pub version_id: String,
    #[serde(default)]
    pub version_number: String,
    pub file_url: String,
    #[serde(default)]
    pub file_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_sha1: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_size: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CastPackSource {
    pub catalog_id: String,
    pub manifest_url: String,
    #[serde(default = "yes")]
    pub autoupdate: bool,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub changelog: String,
    #[serde(default)]
    pub ram_applied: bool,
}

fn yes() -> bool {
    true
}

impl CastPackSource {
    pub fn new(catalog_id: impl Into<String>, manifest_url: impl Into<String>, autoupdate: bool) -> Self {
        Self {
            catalog_id: catalog_id.into(),
            manifest_url: manifest_url.into(),
            autoupdate,
            version: String::new(),
            changelog: String::new(),
            ram_applied: false,
        }
    }

    pub fn is_outdated(&self, available: &str) -> bool {
        !available.trim().is_empty() && available.trim() != self.version.trim()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct InstanceSettings {
    pub override_memory: bool,
    pub min_ram: u32,
    pub max_ram: u32,
    pub override_java: bool,
    pub java_mode: JavaMode,
    pub java_path: String,
}

impl InstanceSettings {
    pub fn apply(&self, base: &AppConfig) -> AppConfig {
        let mut config = base.clone();

        if self.override_memory {
            if self.min_ram > 0 {
                config.java.min_ram = self.min_ram;
            }
            if self.max_ram > 0 {
                config.java.max_ram = self.max_ram;
            }
        }

        if self.override_java {
            config.java.java_mode = self.java_mode;
            config.java.java_path = self.java_path.trim().to_string();

            if config.java.java_mode == JavaMode::Manual && config.java.java_path.is_empty() {
                config.java.java_mode = JavaMode::Auto;
            }
        }

        config
    }

    pub fn overrides_anything(&self) -> bool {
        self.override_memory || self.override_java
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Playtime {
    pub total_seconds: u64,
    pub last_seconds: u64,
    pub last_played_at: u64,
}

impl Playtime {
    pub fn started(&mut self, at_millis: u64) {
        self.last_played_at = at_millis;
    }

    pub fn finished(&mut self, seconds: u64) {
        self.last_seconds = seconds;
        self.total_seconds = self.total_seconds.saturating_add(seconds);
    }

    pub fn session_seconds(started_at: u64, ended_at: u64) -> u64 {
        ended_at.saturating_sub(started_at) / 1000
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Instance {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub minecraft_version: String,
    #[serde(default)]
    pub icon: String,
    #[serde(rename = "type")]
    pub loader: LoaderType,
    #[serde(default)]
    pub installed: bool,
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loader_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pack: Option<PackSource>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub castpack: Option<CastPackSource>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local_pack: Option<LocalPackSource>,
    #[serde(default)]
    pub settings: InstanceSettings,
    #[serde(default)]
    pub playtime: Playtime,
    #[serde(skip)]
    pub dir: String,
}

fn default_version() -> u32 {
    1
}

impl Instance {
    pub fn effective_config(&self, base: &AppConfig) -> AppConfig {
        self.settings.apply(base)
    }

    pub fn require_loader_version(&self) -> CommandResult<&str> {
        self.loader_version.as_deref().filter(|v| !v.is_empty()).ok_or_else(|| {
            CommandError::manifest(format!(
                "У сборки «{}» не указана версия {}",
                self.name,
                self.loader.label()
            ))
        })
    }
}

#[derive(Default)]
pub struct InstanceRegistry {
    instances: RwLock<HashMap<String, Instance>>,
}

impl InstanceRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn all(&self) -> Vec<Instance> {
        let instances = self.instances.read().await;
        let mut list: Vec<Instance> = instances.values().cloned().collect();
        list.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        list
    }

    pub async fn get(&self, id: &str) -> CommandResult<Instance> {
        self.instances
            .read()
            .await
            .get(id)
            .cloned()
            .ok_or_else(|| CommandError::unknown(format!("Сборка {id} не найдена")))
    }

    pub async fn reload(&self, paths: &LauncherPaths) -> CommandResult<Vec<Instance>> {
        let root = paths.instances_root();
        crate::fs_util::ensure_dir(&root).await?;

        let mut entries = tokio::fs::read_dir(&root)
            .await
            .map_err(|e| CommandError::io("Не удалось прочитать каталог сборок", &root, e))?;

        let mut loaded = HashMap::new();

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| CommandError::io("Не удалось прочитать каталог сборок", &root, e))?
        {
            if !entry.file_type().await.map(|kind| kind.is_dir()).unwrap_or(false) {
                continue;
            }

            let dir = entry.path();
            let Some(instance) = load_from_dir(&dir).await else { continue };

            loaded.insert(instance.id.clone(), instance);
        }

        *self.instances.write().await = loaded;

        Ok(self.all().await)
    }

    pub async fn create(&self, paths: &LauncherPaths, mut instance: Instance) -> CommandResult<Instance> {
        let mut dir = paths.instances_root().join(&instance.id);

        if dir.exists() {
            let suffix = uuid::Uuid::new_v4().simple().to_string();
            instance.id = format!("{}-{}", instance.id, &suffix[..8]);
            dir = paths.instances_root().join(&instance.id);
        }

        instance.installed = false;
        instance.dir = dir.display().to_string();

        write_json_atomic(&dir.join("instance.json"), &instance).await?;

        self.instances
            .write()
            .await
            .insert(instance.id.clone(), instance.clone());

        Ok(instance)
    }

    pub async fn update<F>(&self, paths: &LauncherPaths, id: &str, apply: F) -> CommandResult<Instance>
    where
        F: FnOnce(&mut Instance),
    {
        let mut instances = self.instances.write().await;

        let instance = instances
            .get_mut(id)
            .ok_or_else(|| CommandError::unknown(format!("Сборка {id} не найдена")))?;

        apply(instance);

        let updated = instance.clone();
        drop(instances);

        write_json_atomic(&paths.instance(id).config_file(), &updated).await?;

        Ok(updated)
    }

    pub async fn mark_installed(&self, paths: &LauncherPaths, id: &str) -> CommandResult<Instance> {
        self.update(paths, id, |instance| instance.installed = true).await
    }

    pub async fn record_launch(
        &self,
        paths: &LauncherPaths,
        id: &str,
        at_millis: u64,
    ) -> CommandResult<Instance> {
        self.update(paths, id, |instance| instance.playtime.started(at_millis))
            .await
    }

    pub async fn record_session(
        &self,
        paths: &LauncherPaths,
        id: &str,
        seconds: u64,
    ) -> CommandResult<Instance> {
        self.update(paths, id, |instance| instance.playtime.finished(seconds))
            .await
    }

    pub async fn remove(&self, paths: &LauncherPaths, id: &str) -> CommandResult<()> {
        let dir = paths.instance(id).root().to_path_buf();

        tokio::fs::remove_dir_all(&dir)
            .await
            .map_err(|e| CommandError::io("Не удалось удалить каталог сборки", &dir, e))?;

        self.instances.write().await.remove(id);

        Ok(())
    }
}

async fn load_from_dir(dir: &Path) -> Option<Instance> {
    let file = dir.join("instance.json");
    let mut instance: Instance = read_json_opt(&file).await?;

    instance.dir = dir.display().to_string();

    if instance.id.trim().is_empty() {
        eprintln!("Пропускаю сборку без id: {}", file.display());
        return None;
    }

    Some(instance)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn instance_json_format_is_unchanged() {
        let instance: Instance = serde_json::from_value(json!({
            "id": "abc",
            "name": "Моя сборка",
            "description": "",
            "minecraftVersion": "1.20.1",
            "type": "forge",
            "installed": true,
            "version": 1,
            "loaderVersion": "1.20.1-47.2.0",
            "pendingInstall": true
        }))
        .unwrap();

        assert_eq!(instance.loader, LoaderType::Forge);
        assert_eq!(instance.require_loader_version().unwrap(), "1.20.1-47.2.0");

        let written = serde_json::to_value(&instance).unwrap();
        assert!(written.get("pendingInstall").is_none());
        assert_eq!(written["type"], "forge");
        assert_eq!(written["minecraftVersion"], "1.20.1");
    }

    #[test]
    fn pack_source_survives_a_json_round_trip() {
        let instance: Instance = serde_json::from_value(json!({
            "id": "abc",
            "name": "Fabulously Optimized",
            "minecraftVersion": "1.20.1",
            "type": "fabric",
            "pack": {
                "provider": "modrinth",
                "projectId": "1KVo5zza",
                "versionId": "abcdefgh",
                "versionNumber": "5.4.0",
                "fileUrl": "https://cdn.modrinth.com/pack.mrpack",
                "fileName": "pack.mrpack",
                "fileSha1": "aaa",
                "fileSize": 4096
            }
        }))
        .unwrap();

        let pack = instance.pack.clone().unwrap();
        assert_eq!(pack.provider, PackProvider::Modrinth);
        assert_eq!(pack.project_id, "1KVo5zza");

        let written = serde_json::to_value(&instance).unwrap();
        assert_eq!(written["pack"]["provider"], "modrinth");
        assert_eq!(written["pack"]["fileUrl"], "https://cdn.modrinth.com/pack.mrpack");

        let parsed: Instance = serde_json::from_value(written).unwrap();
        assert_eq!(parsed.pack, instance.pack);
    }

    #[test]
    fn a_pack_brought_as_a_file_survives_a_json_round_trip() {
        let instance: Instance = serde_json::from_value(json!({
            "id": "abc",
            "name": "TerraFirmaGreg",
            "minecraftVersion": "1.20.1",
            "type": "forge",
            "localPack": {
                "kind": "multimc",
                "name": "TerraFirmaGreg",
                "version": "0.9.5"
            }
        }))
        .unwrap();

        let source = instance.local_pack.clone().unwrap();
        assert_eq!(source.kind, LocalPackKind::MultiMc);
        assert_eq!(source.name, "TerraFirmaGreg");
        assert!(instance.pack.is_none() && instance.castpack.is_none());

        let written = serde_json::to_value(&instance).unwrap();
        assert_eq!(written["localPack"]["kind"], "multimc");
        assert_eq!(written["localPack"]["version"], "0.9.5");

        let parsed: Instance = serde_json::from_value(written).unwrap();
        assert_eq!(parsed.local_pack, instance.local_pack);
    }

    #[test]
    fn a_pack_kind_says_whether_its_files_still_have_to_be_looked_up() {
        assert!(LocalPackKind::CurseForge.resolves_files(), "в архиве только ссылки на моды");
        assert!(!LocalPackKind::Modrinth.resolves_files());
        assert!(!LocalPackKind::MultiMc.resolves_files(), "моды уже лежат внутри");

        for kind in LocalPackKind::ALL {
            assert_eq!(
                serde_json::to_value(kind).unwrap(),
                serde_json::json!(kind.key()),
                "{kind:?}"
            );
        }
    }

    #[test]
    fn a_castpack_source_survives_a_json_round_trip() {
        let instance: Instance = serde_json::from_value(json!({
            "id": "castpack-rpg",
            "name": "zaralX RPG",
            "minecraftVersion": "1.20.1",
            "type": "forge",
            "castpack": {
                "catalogId": "zaralx-rpg",
                "manifestUrl": "https://cdn.zaralx.ru/packs/rpg/manifest.json",
                "autoupdate": false,
                "version": "1.4.2",
                "changelog": "Убран OptiFine",
                "ramApplied": true
            }
        }))
        .unwrap();

        let source = instance.castpack.clone().unwrap();
        assert!(!source.autoupdate);
        assert_eq!(source.version, "1.4.2");

        let written = serde_json::to_value(&instance).unwrap();
        assert_eq!(written["castpack"]["catalogId"], "zaralx-rpg");
        assert_eq!(written["castpack"]["ramApplied"], true);

        let parsed: Instance = serde_json::from_value(written).unwrap();
        assert_eq!(parsed.castpack, instance.castpack);
    }

    #[test]
    fn a_castpack_source_written_without_flags_updates_itself() {
        let source: CastPackSource = serde_json::from_value(json!({
            "catalogId": "zaralx-rpg",
            "manifestUrl": "https://cdn.zaralx.ru/packs/rpg/manifest.json"
        }))
        .unwrap();

        assert!(source.autoupdate, "по умолчанию сборка обновляется");
        assert!(!source.ram_applied);
        assert!(source.version.is_empty());
    }

    #[test]
    fn a_new_version_in_the_catalog_is_what_makes_a_pack_outdated() {
        let mut source = CastPackSource::new("rpg", "https://cdn.zaralx.ru/m.json", true);
        source.version = "1.4.2".into();

        assert!(source.is_outdated("1.5.0"));
        assert!(!source.is_outdated("1.4.2"));
        assert!(!source.is_outdated("  1.4.2  "));
        assert!(!source.is_outdated("  "), "пустая версия ничего не говорит об обновлении");

        assert!(
            source.is_outdated("1.0.0"),
            "откат автора - тоже повод переустановить: версии не сравниваем, а сверяем"
        );
    }

    #[test]
    fn instances_without_a_pack_do_not_gain_the_field() {
        let instance: Instance = serde_json::from_value(json!({
            "id": "abc",
            "name": "Своя сборка",
            "minecraftVersion": "1.20.1",
            "type": "vanilla"
        }))
        .unwrap();

        assert!(instance.pack.is_none());
        assert!(instance.castpack.is_none());
        assert!(instance.local_pack.is_none());

        let written = serde_json::to_value(&instance).unwrap();
        assert!(written.get("pack").is_none());
        assert!(written.get("castpack").is_none());
        assert!(written.get("localPack").is_none());
    }

    #[test]
    fn settings_are_empty_for_instances_written_before_they_existed() {
        let instance: Instance = serde_json::from_value(json!({
            "id": "abc",
            "name": "Старая сборка",
            "minecraftVersion": "1.20.1",
            "type": "vanilla"
        }))
        .unwrap();

        assert!(!instance.settings.overrides_anything());

        let base = base_config();
        let effective = instance.effective_config(&base);

        assert_eq!(effective.java.min_ram, base.java.min_ram);
        assert_eq!(effective.java.java_mode, base.java.java_mode);
    }

    #[test]
    fn instance_overrides_replace_only_enabled_groups() {
        let settings = InstanceSettings {
            override_memory: true,
            min_ram: 2048,
            max_ram: 8192,
            ..Default::default()
        };

        let base = base_config();
        let effective = settings.apply(&base);

        assert_eq!(effective.heap_args(), vec!["-Xms2048M", "-Xmx8192M"]);
        assert_eq!(effective.java.java_path, base.java.java_path);
        assert_eq!(effective.java.java_mode, base.java.java_mode);
    }

    #[test]
    fn zero_memory_values_fall_back_to_global_config() {
        let settings = InstanceSettings {
            override_memory: true,
            max_ram: 6144,
            ..Default::default()
        };

        let effective = settings.apply(&base_config());

        assert_eq!(effective.java.min_ram, 1024);
        assert_eq!(effective.java.max_ram, 6144);
    }

    #[test]
    fn instance_java_override_switches_runtime() {
        let settings = InstanceSettings {
            override_java: true,
            java_mode: JavaMode::Manual,
            java_path: "  C:\\jdk21\\bin\\javaw.exe  ".into(),
            ..Default::default()
        };

        let effective = settings.apply(&base_config());

        assert_eq!(effective.java.java_mode, JavaMode::Manual);
        assert_eq!(effective.manual_java_path(), Some("C:\\jdk21\\bin\\javaw.exe"));
    }

    #[test]
    fn manual_override_without_path_falls_back_to_auto() {
        let settings = InstanceSettings {
            override_java: true,
            java_mode: JavaMode::Manual,
            java_path: "   ".into(),
            ..Default::default()
        };

        let effective = settings.apply(&base_config());

        assert_eq!(effective.java.java_mode, JavaMode::Auto);
        assert_eq!(effective.manual_java_path(), None);
    }

    #[test]
    fn settings_survive_a_json_round_trip() {
        let instance: Instance = serde_json::from_value(json!({
            "id": "abc",
            "name": "Сборка",
            "minecraftVersion": "1.20.1",
            "type": "fabric",
            "settings": {
                "overrideMemory": true,
                "minRam": 3072,
                "maxRam": 6144,
                "overrideJava": true,
                "javaMode": "system"
            }
        }))
        .unwrap();

        let written = serde_json::to_value(&instance).unwrap();

        assert_eq!(written["settings"]["minRam"], 3072);
        assert_eq!(written["settings"]["javaMode"], "system");
        assert_eq!(written["settings"]["javaPath"], "");

        let parsed: Instance = serde_json::from_value(written).unwrap();
        assert_eq!(parsed.settings, instance.settings);
    }

    fn base_config() -> AppConfig {
        AppConfig::defaults(std::path::Path::new("/cfg"))
    }

    #[test]
    fn instances_written_before_the_counter_existed_start_from_zero() {
        let instance: Instance = serde_json::from_value(json!({
            "id": "abc",
            "name": "Старая сборка",
            "minecraftVersion": "1.20.1",
            "type": "vanilla"
        }))
        .unwrap();

        assert_eq!(instance.playtime, Playtime::default());
        assert_eq!(serde_json::to_value(&instance).unwrap()["playtime"]["totalSeconds"], 0);
    }

    #[test]
    fn every_session_adds_up_to_the_total() {
        let mut playtime = Playtime::default();

        playtime.started(1_700_000_000_000);
        playtime.finished(3600);

        assert_eq!(playtime.total_seconds, 3600);
        assert_eq!(playtime.last_seconds, 3600);

        playtime.finished(600);

        assert_eq!(playtime.total_seconds, 4200);
        assert_eq!(playtime.last_seconds, 600, "последняя сессия перезаписывается");
        assert_eq!(playtime.last_played_at, 1_700_000_000_000, "запуск отмечен один раз");
    }

    #[test]
    fn a_session_is_measured_in_whole_seconds() {
        assert_eq!(Playtime::session_seconds(1_000, 91_500), 90);
        assert_eq!(Playtime::session_seconds(1_000, 1_400), 0);
        assert_eq!(
            Playtime::session_seconds(5_000, 1_000),
            0,
            "переведённые назад часы не должны отматывать счётчик"
        );
    }

    #[test]
    fn playtime_survives_a_json_round_trip() {
        let instance: Instance = serde_json::from_value(json!({
            "id": "abc",
            "name": "Сборка",
            "minecraftVersion": "1.20.1",
            "type": "fabric",
            "playtime": {
                "totalSeconds": 123456,
                "lastSeconds": 780,
                "lastPlayedAt": 1_700_000_000_000u64
            }
        }))
        .unwrap();

        let written = serde_json::to_value(&instance).unwrap();

        assert_eq!(written["playtime"]["totalSeconds"], 123456);
        assert_eq!(written["playtime"]["lastPlayedAt"], 1_700_000_000_000u64);

        let parsed: Instance = serde_json::from_value(written).unwrap();
        assert_eq!(parsed.playtime, instance.playtime);
    }

    #[test]
    fn missing_loader_version_is_reported_with_instance_name() {
        let instance: Instance = serde_json::from_value(json!({
            "id": "abc",
            "name": "Без версии",
            "minecraftVersion": "1.20.1",
            "type": "fabric"
        }))
        .unwrap();

        let error = instance.require_loader_version().unwrap_err();
        assert!(error.message.contains("Без версии"));
        assert!(error.message.contains("Fabric"));
    }
}
