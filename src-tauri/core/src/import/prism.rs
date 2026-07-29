use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::config::JavaMode;
use crate::error::{CommandError, CommandResult};
use crate::instance::{Instance, InstanceSettings, LoaderType};
use crate::paths::LauncherPaths;

use super::copy::{self, Progress};
use super::ini::Ini;
use super::ImportOptions;

pub const INSTANCES: &str = "instances";
pub const LIBRARIES: &str = "libraries";
pub const ASSETS: &str = "assets";
pub const ICONS: &str = "icons";
pub const JAVA: &str = "java";

pub const CONFIG_FILE: &str = "instance.cfg";
pub const PACK_FILE: &str = "mmc-pack.json";

const MINECRAFT_UID: &str = "net.minecraft";

const LOADERS: &[(&str, &str, Option<LoaderType>)] = &[
    ("net.fabricmc.fabric-loader", "Fabric", Some(LoaderType::Fabric)),
    ("net.minecraftforge", "Forge", Some(LoaderType::Forge)),
    ("net.neoforged", "NeoForge", None),
    ("org.quiltmc.quilt-loader", "Quilt", None),
    ("com.mumfrey.liteloader", "LiteLoader", None),
];

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
    pub icon_key: String,
    pub settings: InstanceSettings,
    pub pack: Option<ManagedPack>,
    pub blocked: Option<String>,
}

impl ScannedInstance {
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
            settings: self.settings.clone(),
            dir: String::new(),
        })
    }
}

pub fn data_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    if cfg!(target_os = "windows") {
        if let Ok(appdata) = std::env::var("APPDATA") {
            dirs.push(PathBuf::from(appdata).join("PrismLauncher"));
        }
    } else if cfg!(target_os = "macos") {
        if let Ok(home) = std::env::var("HOME") {
            dirs.push(
                PathBuf::from(home)
                    .join("Library")
                    .join("Application Support")
                    .join("PrismLauncher"),
            );
        }
    } else {
        if let Ok(data_home) = std::env::var("XDG_DATA_HOME") {
            dirs.push(PathBuf::from(data_home).join("PrismLauncher"));
        }

        if let Ok(home) = std::env::var("HOME") {
            let home = PathBuf::from(home);

            dirs.push(home.join(".local").join("share").join("PrismLauncher"));
            dirs.push(
                home.join(".var")
                    .join("app")
                    .join("org.prismlauncher.PrismLauncher")
                    .join("data")
                    .join("PrismLauncher"),
            );
        }
    }

    dirs
}

pub fn is_data_dir(dir: &Path) -> bool {
    dir.join(INSTANCES).is_dir()
}

pub fn detect() -> Option<PathBuf> {
    data_dirs().into_iter().find(|dir| is_data_dir(dir))
}

pub fn normalize(dir: &Path) -> PathBuf {
    if is_data_dir(dir) {
        return dir.to_path_buf();
    }

    if dir.file_name().is_some_and(|name| name == INSTANCES) {
        if let Some(parent) = dir.parent() {
            return parent.to_path_buf();
        }
    }

    let portable = dir.join("UserData");

    if is_data_dir(&portable) {
        return portable;
    }

    dir.to_path_buf()
}

pub fn game_dir(instance_dir: &Path) -> PathBuf {
    let dotted = instance_dir.join(".minecraft");
    let plain = instance_dir.join("minecraft");

    if dotted.is_dir() && !plain.is_dir() {
        dotted
    } else {
        plain
    }
}

pub fn client_jar(root: &Path, minecraft_version: &str) -> PathBuf {
    root.join(LIBRARIES)
        .join("com")
        .join("mojang")
        .join("minecraft")
        .join(minecraft_version)
        .join(format!("minecraft-{minecraft_version}-client.jar"))
}

pub fn forge_installer_target(paths: &LauncherPaths, instance: &Instance) -> Option<PathBuf> {
    if instance.loader != LoaderType::Forge {
        return None;
    }

    instance
        .loader_version
        .as_deref()
        .map(|version| paths.forge_cache(version).installer_jar())
}

pub fn forge_installer(root: &Path, forge_version: &str) -> PathBuf {
    root.join(LIBRARIES)
        .join("net")
        .join("minecraftforge")
        .join("forge")
        .join(forge_version)
        .join(format!("forge-{forge_version}-installer.jar"))
}

#[derive(Debug, Clone)]
pub struct SharedTargets {
    pub libraries: PathBuf,
    pub asset_indexes: PathBuf,
    pub asset_objects: PathBuf,
    pub java_runtimes: PathBuf,
}

#[derive(Debug, Clone)]
pub struct InstanceTargets {
    pub minecraft: PathBuf,
    pub client_jar: PathBuf,
    pub forge_installer: Option<PathBuf>,
}

pub async fn copy_shared(
    root: &Path,
    options: &ImportOptions,
    targets: &SharedTargets,
    progress: &Progress<'_>,
    on_step: impl Fn(&str),
) -> CommandResult<()> {
    if options.libraries {
        on_step("Библиотеки");
        copy::merge_dir(&root.join(LIBRARIES), &targets.libraries, progress).await?;
    }

    if options.assets {
        let assets = root.join(ASSETS);

        on_step("Ресурсы игры");
        copy::merge_dir(&assets.join("indexes"), &targets.asset_indexes, progress).await?;
        copy::merge_dir(&assets.join("objects"), &targets.asset_objects, progress).await?;
    }

    if options.java {
        on_step("Java");
        copy::merge_dir(&root.join(JAVA), &targets.java_runtimes, progress).await?;
    }

    Ok(())
}

pub async fn copy_instance(
    root: &Path,
    scanned: &ScannedInstance,
    targets: &InstanceTargets,
    progress: &Progress<'_>,
) -> CommandResult<()> {
    let source = game_dir(&root.join(INSTANCES).join(&scanned.folder));

    copy::merge_dir(&source, &targets.minecraft, progress).await?;

    copy::copy_file(
        &client_jar(root, &scanned.minecraft_version),
        &targets.client_jar,
        progress,
    )
    .await?;

    if let (Some(target), Some(version)) = (&targets.forge_installer, &scanned.loader_version) {
        copy::copy_file(&forge_installer(root, version), target, progress).await?;
    }

    Ok(())
}

pub async fn scan(root: &Path) -> CommandResult<Vec<ScannedInstance>> {
    let instances = root.join(INSTANCES);

    if !instances.is_dir() {
        return Err(CommandError::fs(format!(
            "В каталоге нет папки instances: {}",
            root.display()
        )));
    }

    let mut entries = tokio::fs::read_dir(&instances)
        .await
        .map_err(|e| CommandError::io("Не удалось прочитать сборки Prism", &instances, e))?;

    let forge_versions = forge_versions(root).await;
    let mut found = Vec::new();

    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|e| CommandError::io("Не удалось прочитать сборки Prism", &instances, e))?
    {
        let dir = entry.path();
        let config = dir.join(CONFIG_FILE);

        if !config.is_file() {
            continue;
        }

        let folder = entry.file_name().to_string_lossy().to_string();
        let config = tokio::fs::read_to_string(&config).await.unwrap_or_default();
        let pack = tokio::fs::read_to_string(dir.join(PACK_FILE)).await.unwrap_or_default();

        let mut scanned = parse(&folder, &config, &pack);

        scanned.icon = find_icon(root, &scanned.icon_key);

        if scanned.loader == Some(LoaderType::Forge) {
            scanned.loader_version = scanned
                .loader_version
                .map(|version| pick_forge_version(&forge_versions, &version));
        }

        found.push(scanned);
    }

    found.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    Ok(found)
}

pub fn parse(folder: &str, config: &str, pack: &str) -> ScannedInstance {
    let ini = Ini::parse(config);
    let general = ini.general();

    let name = match general.string("name") {
        name if name.is_empty() => folder.to_string(),
        name => name,
    };

    let mut scanned = ScannedInstance {
        folder: folder.to_string(),
        name,
        description: general.string("notes"),
        minecraft_version: String::new(),
        loader: None,
        loader_version: None,
        loader_label: "Vanilla".into(),
        icon: None,
        icon_key: general.string("iconKey"),
        settings: settings(&ini),
        pack: managed_pack(&ini),
        blocked: None,
    };

    let components = match parse_components(pack) {
        Ok(components) => components,
        Err(reason) => {
            scanned.blocked = Some(reason);
            return scanned;
        }
    };

    let Some(minecraft) = version_of(&components, MINECRAFT_UID) else {
        scanned.blocked = Some("в mmc-pack.json нет версии Minecraft".into());
        return scanned;
    };

    scanned.minecraft_version = minecraft.clone();

    let Some((uid, label, kind)) = find_loader(&components) else {
        scanned.loader = Some(LoaderType::Vanilla);
        return scanned;
    };

    let version = version_of(&components, &uid);

    scanned.loader_label = match &version {
        Some(version) => format!("{label} {version}"),
        None => label.to_string(),
    };

    let Some(kind) = kind else {
        scanned.blocked = Some(format!("лаунчер пока не умеет {label}"));
        return scanned;
    };

    let Some(version) = version else {
        scanned.blocked = Some(format!("в mmc-pack.json нет версии {label}"));
        return scanned;
    };

    scanned.loader = Some(kind);
    scanned.loader_version = Some(match kind {
        LoaderType::Forge => forge_maven_version(&minecraft, &version),
        _ => version,
    });

    scanned
}

fn settings(ini: &Ini) -> InstanceSettings {
    let general = ini.general();

    let override_memory = general.flag("OverrideMemory");
    let java_path = general.string("JavaPath");

    let override_java =
        general.flag("OverrideJavaLocation") && !general.flag("AutomaticJava") && !java_path.is_empty();

    InstanceSettings {
        override_memory,
        min_ram: override_memory.then(|| general.number("MinMemAlloc")).flatten().unwrap_or(0),
        max_ram: override_memory.then(|| general.number("MaxMemAlloc")).flatten().unwrap_or(0),
        override_java,
        java_mode: if override_java { JavaMode::Manual } else { JavaMode::default() },
        java_path: if override_java { java_path } else { String::new() },
    }
}

fn managed_pack(ini: &Ini) -> Option<ManagedPack> {
    let general = ini.general();

    if !general.flag("ManagedPack") {
        return None;
    }

    let provider = general.string("ManagedPackType");
    let project_id = general.string("ManagedPackID");

    if provider.is_empty() || project_id.is_empty() {
        return None;
    }

    Some(ManagedPack {
        provider,
        project_id,
        version_id: general.string("ManagedPackVersionID"),
        version_name: general.string("ManagedPackVersionName"),
        name: general.string("ManagedPackName"),
    })
}

pub fn find_icon(root: &Path, icon_key: &str) -> Option<String> {
    if icon_key.is_empty() || icon_key == "default" {
        return None;
    }

    let dir = root.join(ICONS);

    crate::icons::extensions().into_iter().find_map(|extension| {
        let name = format!("{icon_key}.{extension}");
        dir.join(&name).is_file().then_some(name)
    })
}

#[derive(Debug, Deserialize)]
struct MmcPack {
    #[serde(default)]
    components: Vec<MmcComponent>,
}

#[derive(Debug, Deserialize)]
struct MmcComponent {
    #[serde(default)]
    uid: String,
    #[serde(default)]
    version: Option<String>,
    #[serde(default, rename = "cachedVersion")]
    cached_version: Option<String>,
}

fn parse_components(pack: &str) -> Result<Vec<MmcComponent>, String> {
    if pack.trim().is_empty() {
        return Err("рядом со сборкой нет mmc-pack.json".into());
    }

    serde_json::from_str::<MmcPack>(pack)
        .map(|pack| pack.components)
        .map_err(|_| "mmc-pack.json не читается".to_string())
}

fn version_of(components: &[MmcComponent], uid: &str) -> Option<String> {
    components
        .iter()
        .find(|component| component.uid == uid)
        .and_then(|component| {
            component
                .version
                .clone()
                .or_else(|| component.cached_version.clone())
        })
        .map(|version| version.trim().to_string())
        .filter(|version| !version.is_empty())
}

fn find_loader(components: &[MmcComponent]) -> Option<(String, &'static str, Option<LoaderType>)> {
    components.iter().find_map(|component| {
        LOADERS
            .iter()
            .find(|(uid, _, _)| *uid == component.uid)
            .map(|(uid, label, kind)| ((*uid).to_string(), *label, *kind))
    })
}

fn forge_maven_version(minecraft: &str, forge: &str) -> String {
    if forge.starts_with(&format!("{minecraft}-")) {
        return forge.to_string();
    }

    format!("{minecraft}-{forge}")
}

pub fn pick_forge_version(available: &[String], guess: &str) -> String {
    if available.iter().any(|version| version == guess) {
        return guess.to_string();
    }

    available
        .iter()
        .find(|version| version.starts_with(&format!("{guess}-")))
        .cloned()
        .unwrap_or_else(|| guess.to_string())
}

async fn forge_versions(root: &Path) -> Vec<String> {
    let dir = root
        .join(LIBRARIES)
        .join("net")
        .join("minecraftforge")
        .join("forge");

    let Ok(mut entries) = tokio::fs::read_dir(&dir).await else {
        return Vec::new();
    };

    let mut versions = Vec::new();

    while let Ok(Some(entry)) = entries.next_entry().await {
        if entry.file_type().await.map(|kind| kind.is_dir()).unwrap_or(false) {
            versions.push(entry.file_name().to_string_lossy().to_string());
        }
    }

    versions
}

#[cfg(test)]
mod tests {
    use super::*;

    const FABRIC_PACK: &str = r#"{
        "components": [
            { "cachedName": "LWJGL 3", "dependencyOnly": true, "uid": "org.lwjgl3", "version": "3.3.3" },
            { "cachedName": "Minecraft", "important": true, "uid": "net.minecraft", "version": "1.21.11" },
            { "dependencyOnly": true, "uid": "net.fabricmc.intermediary", "version": "1.21.11" },
            { "cachedName": "Fabric Loader", "uid": "net.fabricmc.fabric-loader", "version": "0.18.4" }
        ],
        "formatVersion": 1
    }"#;

    const FABRIC_CONFIG: &str = r#"
[General]
ConfigVersion=1.2
ManagedPack=true
iconKey=modrinth_fabulously-optimized
ManagedPackID=1KVo5zza
ManagedPackType=modrinth
ManagedPackName=Fabulously Optimized
ManagedPackVersionID=fzpQA5K4
ManagedPackVersionName=12.0.5
name=Fabulously Optimized 12.0.5
AutomaticJava=true
InstanceType=OneSix
JavaPath=C:/Users/admin/AppData/Roaming/PrismLauncher/java/java-runtime-delta/bin/javaw.exe
OverrideJavaLocation=true
OverrideMemory=false
notes=Мой любимый пак
"#;

    const FORGE_PACK: &str = r#"{
        "components": [
            { "uid": "org.lwjgl3", "version": "3.3.1", "dependencyOnly": true },
            { "uid": "net.minecraft", "version": "1.20.1", "important": true },
            { "cachedName": "Forge", "uid": "net.minecraftforge", "version": "47.4.13" }
        ],
        "formatVersion": 1
    }"#;

    const NEOFORGE_PACK: &str = r#"{
        "components": [
            { "uid": "net.minecraft", "version": "1.21.1" },
            { "cachedName": "NeoForge", "uid": "net.neoforged", "version": "21.1.213" }
        ]
    }"#;

    #[test]
    fn a_fabric_instance_is_mapped_field_by_field() {
        let scanned = parse("Fabulously Optimized", FABRIC_CONFIG, FABRIC_PACK);

        assert!(scanned.is_importable());
        assert_eq!(scanned.name, "Fabulously Optimized 12.0.5");
        assert_eq!(scanned.description, "Мой любимый пак");
        assert_eq!(scanned.minecraft_version, "1.21.11");
        assert_eq!(scanned.loader, Some(LoaderType::Fabric));
        assert_eq!(scanned.loader_version.as_deref(), Some("0.18.4"));
        assert_eq!(scanned.loader_label, "Fabric 0.18.4");
    }

    #[test]
    fn a_managed_modrinth_pack_is_carried_over() {
        let pack = parse("Fabulously Optimized", FABRIC_CONFIG, FABRIC_PACK)
            .pack
            .unwrap();

        assert_eq!(pack.provider, "modrinth");
        assert_eq!(pack.project_id, "1KVo5zza");
        assert_eq!(pack.version_id, "fzpQA5K4");
        assert_eq!(pack.version_name, "12.0.5");
    }

    #[test]
    fn a_managed_pack_without_identifiers_is_dropped() {
        let config = "[General]\nManagedPack=true\nManagedPackType=modrinth\nManagedPackID=\nname=Create Azure";

        assert!(parse("Create Azure", config, FABRIC_PACK).pack.is_none());
    }

    #[test]
    fn forge_versions_gain_the_minecraft_prefix() {
        let scanned = parse("TerraFirmaGreg", "[General]\nname=TFG", FORGE_PACK);

        assert_eq!(scanned.loader, Some(LoaderType::Forge));
        assert_eq!(scanned.loader_version.as_deref(), Some("1.20.1-47.4.13"));
        assert_eq!(scanned.loader_label, "Forge 47.4.13");
    }

    #[test]
    fn an_already_prefixed_forge_version_is_left_alone() {
        assert_eq!(forge_maven_version("1.20.1", "1.20.1-47.4.13"), "1.20.1-47.4.13");
        assert_eq!(forge_maven_version("1.20.1", "47.4.13"), "1.20.1-47.4.13");
    }

    #[test]
    fn legacy_forge_takes_the_exact_name_from_the_library_folder() {
        let available = vec![
            "1.20.1-47.4.13".to_string(),
            "1.7.10-10.13.4.1614-1.7.10".to_string(),
        ];

        assert_eq!(
            pick_forge_version(&available, "1.7.10-10.13.4.1614"),
            "1.7.10-10.13.4.1614-1.7.10"
        );
        assert_eq!(pick_forge_version(&available, "1.20.1-47.4.13"), "1.20.1-47.4.13");
        assert_eq!(pick_forge_version(&[], "1.20.1-47.4.13"), "1.20.1-47.4.13");
    }

    #[test]
    fn an_instance_without_a_loader_is_vanilla() {
        let pack = r#"{ "components": [ { "uid": "net.minecraft", "version": "1.20.1" } ] }"#;
        let scanned = parse("1.20.1", "[General]\nname=1.20.1", pack);

        assert_eq!(scanned.loader, Some(LoaderType::Vanilla));
        assert_eq!(scanned.loader_label, "Vanilla");
        assert!(scanned.loader_version.is_none());
        assert!(scanned.is_importable());
    }

    #[test]
    fn neoforge_is_recognised_but_refused() {
        let scanned = parse("Create Azure", "[General]\nname=Create Azure", NEOFORGE_PACK);

        assert!(!scanned.is_importable());
        assert_eq!(scanned.loader_label, "NeoForge 21.1.213");
        assert!(scanned.blocked.unwrap().contains("NeoForge"));
        assert_eq!(scanned.minecraft_version, "1.21.1");
    }

    #[test]
    fn a_missing_or_broken_pack_file_blocks_the_instance() {
        assert!(parse("x", "[General]\nname=x", "").blocked.is_some());
        assert!(parse("x", "[General]\nname=x", "{ сломано").blocked.is_some());
        assert!(parse("x", "[General]\nname=x", r#"{"components":[]}"#).blocked.is_some());
    }

    #[test]
    fn a_nameless_instance_falls_back_to_its_folder() {
        let scanned = parse("Моя папка", "[General]\niconKey=default", FABRIC_PACK);

        assert_eq!(scanned.name, "Моя папка");
    }

    #[test]
    fn cached_versions_are_used_when_the_explicit_one_is_absent() {
        let pack = r#"{
            "components": [
                { "uid": "net.minecraft", "cachedVersion": "1.16.5" },
                { "uid": "net.fabricmc.fabric-loader", "cachedVersion": "0.14.9" }
            ]
        }"#;

        let scanned = parse("x", "[General]\nname=x", pack);

        assert_eq!(scanned.minecraft_version, "1.16.5");
        assert_eq!(scanned.loader_version.as_deref(), Some("0.14.9"));
    }

    #[test]
    fn memory_overrides_are_carried_over() {
        let config = "[General]\nname=x\nOverrideMemory=true\nMinMemAlloc=512\nMaxMemAlloc=12544";
        let settings = parse("x", config, FORGE_PACK).settings;

        assert!(settings.override_memory);
        assert_eq!(settings.min_ram, 512);
        assert_eq!(settings.max_ram, 12544);
    }

    #[test]
    fn memory_values_are_ignored_while_the_override_is_off() {
        let config = "[General]\nname=x\nOverrideMemory=false\nMinMemAlloc=512\nMaxMemAlloc=12544";
        let settings = parse("x", config, FORGE_PACK).settings;

        assert!(!settings.overrides_anything());
        assert_eq!(settings.max_ram, 0);
    }

    #[test]
    fn an_automatically_picked_java_is_not_carried_over() {
        let settings = parse("x", FABRIC_CONFIG, FABRIC_PACK).settings;

        assert!(!settings.override_java);
        assert_eq!(settings.java_path, "");
    }

    #[test]
    fn a_java_chosen_by_hand_becomes_a_manual_override() {
        let config = "[General]\nname=x\nOverrideJavaLocation=true\nAutomaticJava=false\nJavaPath=C:/jdk21/bin/javaw.exe";
        let settings = parse("x", config, FORGE_PACK).settings;

        assert!(settings.override_java);
        assert_eq!(settings.java_mode, JavaMode::Manual);
        assert_eq!(settings.java_path, "C:/jdk21/bin/javaw.exe");
    }

    #[test]
    fn conversion_produces_an_uninstalled_instance() {
        let scanned = parse("Fabulously Optimized", FABRIC_CONFIG, FABRIC_PACK);
        let instance = scanned.to_instance("abc".into(), "fo.webp".into()).unwrap();

        assert_eq!(instance.id, "abc");
        assert_eq!(instance.icon, "fo.webp");
        assert_eq!(instance.loader, LoaderType::Fabric);
        assert_eq!(instance.minecraft_version, "1.21.11");
        assert!(!instance.installed, "перенесённое всегда доустанавливается заново");
        assert!(instance.pack.is_none(), "пак проставляется отдельно");
    }

    #[test]
    fn a_blocked_instance_refuses_to_convert() {
        let scanned = parse("Create Azure", "[General]\nname=Create Azure", NEOFORGE_PACK);
        let error = scanned.to_instance("abc".into(), String::new()).unwrap_err();

        assert!(error.message.contains("Create Azure"));
        assert!(error.message.contains("NeoForge"));
    }

    #[test]
    fn a_data_dir_is_recognised_by_its_instances_folder() {
        let root = std::env::temp_dir().join(format!("cast-prism-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(root.join(INSTANCES)).unwrap();

        assert!(is_data_dir(&root));
        assert_eq!(normalize(&root), root);
        assert_eq!(normalize(&root.join(INSTANCES)), root);
        assert!(!is_data_dir(&root.join("нет")));

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn a_portable_folder_resolves_to_its_user_data() {
        let root = std::env::temp_dir().join(format!("cast-prism-{}", uuid::Uuid::new_v4()));
        let user_data = root.join("UserData");
        std::fs::create_dir_all(user_data.join(INSTANCES)).unwrap();

        assert_eq!(normalize(&root), user_data);

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn the_game_folder_may_be_hidden() {
        let root = std::env::temp_dir().join(format!("cast-prism-{}", uuid::Uuid::new_v4()));
        let dotted = root.join("старая");
        let plain = root.join("новая");

        std::fs::create_dir_all(dotted.join(".minecraft")).unwrap();
        std::fs::create_dir_all(plain.join("minecraft")).unwrap();

        assert_eq!(game_dir(&dotted), dotted.join(".minecraft"));
        assert_eq!(game_dir(&plain), plain.join("minecraft"));
        assert_eq!(game_dir(&root.join("пусто")), root.join("пусто").join("minecraft"));

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn icons_are_found_by_any_supported_extension() {
        let root = std::env::temp_dir().join(format!("cast-prism-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(root.join(ICONS)).unwrap();
        std::fs::write(root.join(ICONS).join("modrinth_sop.png"), b"pixels").unwrap();

        assert_eq!(find_icon(&root, "modrinth_sop").as_deref(), Some("modrinth_sop.png"));
        assert!(find_icon(&root, "default").is_none());
        assert!(find_icon(&root, "").is_none());
        assert!(find_icon(&root, "нет-такой").is_none());

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn shared_file_paths_follow_the_prism_layout() {
        let root = Path::new("/prism");

        assert!(client_jar(root, "1.20.1")
            .ends_with(Path::new("com/mojang/minecraft/1.20.1/minecraft-1.20.1-client.jar")));
        assert!(forge_installer(root, "1.20.1-47.4.13").ends_with(Path::new(
            "net/minecraftforge/forge/1.20.1-47.4.13/forge-1.20.1-47.4.13-installer.jar"
        )));
    }

    #[test]
    fn only_a_forge_instance_gets_a_cache_slot_for_its_installer() {
        let paths = LauncherPaths::new(PathBuf::from("/cfg"), None);

        let forge = parse("x", "[General]\nname=x", FORGE_PACK)
            .to_instance("abc".into(), String::new())
            .unwrap();

        let target = forge_installer_target(&paths, &forge).expect("forge-сборке нужен установщик");
        assert!(target.ends_with(Path::new("forge/1.20.1-47.4.13/installer.jar")));

        let fabric = parse("x", "[General]\nname=x", FABRIC_PACK)
            .to_instance("abc".into(), String::new())
            .unwrap();

        assert!(forge_installer_target(&paths, &fabric).is_none());

        let mut without_version = forge.clone();
        without_version.loader_version = None;

        assert!(forge_installer_target(&paths, &without_version).is_none());
    }

    #[tokio::test]
    async fn scanning_reads_every_folder_with_a_config() {
        let root = std::env::temp_dir().join(format!("cast-prism-{}", uuid::Uuid::new_v4()));
        let instances = root.join(INSTANCES);

        for (folder, config, pack) in [
            ("Fabulously Optimized", FABRIC_CONFIG, FABRIC_PACK),
            ("TerraFirmaGreg", "[General]\nname=TerraFirmaGreg", FORGE_PACK),
            ("Create Azure", "[General]\nname=Create Azure", NEOFORGE_PACK),
        ] {
            let dir = instances.join(folder);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join(CONFIG_FILE), config).unwrap();
            std::fs::write(dir.join(PACK_FILE), pack).unwrap();
        }

        // Каталог без instance.cfg — не сборка.
        std::fs::create_dir_all(instances.join("мусор")).unwrap();
        std::fs::write(instances.join("instgroups.json"), b"{}").unwrap();

        std::fs::create_dir_all(root.join(ICONS)).unwrap();
        std::fs::write(
            root.join(ICONS).join("modrinth_fabulously-optimized.webp"),
            b"pixels",
        )
        .unwrap();

        let found = scan(&root).await.unwrap();

        assert_eq!(found.len(), 3);
        assert_eq!(found[0].name, "Create Azure");
        assert!(!found[0].is_importable());

        let fabulously = &found[1];
        assert_eq!(fabulously.name, "Fabulously Optimized 12.0.5");
        assert_eq!(
            fabulously.icon.as_deref(),
            Some("modrinth_fabulously-optimized.webp")
        );

        assert_eq!(found[2].loader_version.as_deref(), Some("1.20.1-47.4.13"));

        std::fs::remove_dir_all(&root).ok();
    }

    /// Раскладывает мини-Prism: сборка с миром, общий клиент и патченый Forge.
    fn prism_tree() -> PathBuf {
        let root = std::env::temp_dir().join(format!("cast-prism-{}", uuid::Uuid::new_v4()));

        let instance = root.join(INSTANCES).join("TerraFirmaGreg");
        let saves = instance.join("minecraft").join("saves").join("Мир");
        std::fs::create_dir_all(&saves).unwrap();
        std::fs::write(instance.join(CONFIG_FILE), "[General]\nname=TFG").unwrap();
        std::fs::write(instance.join(PACK_FILE), FORGE_PACK).unwrap();
        std::fs::write(saves.join("level.dat"), "мир").unwrap();
        std::fs::write(instance.join("minecraft").join("options.txt"), "fov:80").unwrap();

        let client = root
            .join(LIBRARIES)
            .join("com")
            .join("mojang")
            .join("minecraft")
            .join("1.20.1");
        std::fs::create_dir_all(&client).unwrap();
        std::fs::write(client.join("minecraft-1.20.1-client.jar"), "client").unwrap();

        let forge = root
            .join(LIBRARIES)
            .join("net")
            .join("minecraftforge")
            .join("forge")
            .join("1.20.1-47.4.13");
        std::fs::create_dir_all(&forge).unwrap();
        std::fs::write(forge.join("forge-1.20.1-47.4.13-installer.jar"), "installer").unwrap();
        std::fs::write(forge.join("forge-1.20.1-47.4.13-client.jar"), "patched").unwrap();

        let objects = root.join(ASSETS).join("objects").join("ab");
        std::fs::create_dir_all(&objects).unwrap();
        std::fs::create_dir_all(root.join(ASSETS).join("indexes")).unwrap();
        std::fs::write(objects.join("abcdef"), "звук").unwrap();
        std::fs::write(root.join(ASSETS).join("indexes").join("5.json"), "{}").unwrap();

        std::fs::create_dir_all(root.join(JAVA).join("java-runtime-gamma").join("bin")).unwrap();
        std::fs::write(
            root.join(JAVA).join("java-runtime-gamma").join("bin").join("javaw.exe"),
            "java",
        )
        .unwrap();

        root
    }

    fn silent() -> impl Fn(copy::CopyStats) + Send + Sync {
        |_| {}
    }

    fn never() -> impl Fn() -> bool + Send + Sync {
        || false
    }

    #[tokio::test]
    async fn shared_folders_land_in_our_own_layout() {
        let root = prism_tree();
        let to = root.join("наш");

        let targets = SharedTargets {
            libraries: to.join("libraries"),
            asset_indexes: to.join("assets").join("indexes"),
            asset_objects: to.join("assets").join("objects"),
            java_runtimes: to.join("runtime"),
        };

        let on_change = silent();
        let cancelled = never();
        let progress = Progress::new(&on_change, &cancelled);

        let steps = std::sync::Mutex::new(Vec::new());
        copy_shared(&root, &ImportOptions::default(), &targets, &progress, |step| {
            steps.lock().unwrap().push(step.to_string())
        })
        .await
        .unwrap();

        assert_eq!(
            *steps.lock().unwrap(),
            vec!["Библиотеки", "Ресурсы игры", "Java"]
        );
        assert!(targets.asset_objects.join("ab").join("abcdef").is_file());
        assert!(targets.asset_indexes.join("5.json").is_file());
        assert!(targets
            .java_runtimes
            .join("java-runtime-gamma")
            .join("bin")
            .join("javaw.exe")
            .is_file());
        assert!(targets
            .libraries
            .join("net/minecraftforge/forge/1.20.1-47.4.13/forge-1.20.1-47.4.13-client.jar".replace('/', std::path::MAIN_SEPARATOR_STR))
            .is_file());

        std::fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn switched_off_folders_are_left_behind() {
        let root = prism_tree();
        let to = root.join("наш");

        let targets = SharedTargets {
            libraries: to.join("libraries"),
            asset_indexes: to.join("assets").join("indexes"),
            asset_objects: to.join("assets").join("objects"),
            java_runtimes: to.join("runtime"),
        };

        let on_change = silent();
        let cancelled = never();
        let progress = Progress::new(&on_change, &cancelled);

        let options = ImportOptions {
            libraries: true,
            assets: false,
            java: false,
            ..ImportOptions::default()
        };

        copy_shared(&root, &options, &targets, &progress, |_| {}).await.unwrap();

        assert!(targets.libraries.is_dir());
        assert!(!targets.asset_objects.exists());
        assert!(!targets.java_runtimes.exists());

        std::fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn an_instance_arrives_with_its_world_client_and_forge_installer() {
        let root = prism_tree();
        let to = root.join("наш").join("instances").join("abc");

        let scanned = scan(&root).await.unwrap().pop().unwrap();
        assert_eq!(scanned.loader_version.as_deref(), Some("1.20.1-47.4.13"));

        let targets = InstanceTargets {
            minecraft: to.join("minecraft"),
            client_jar: to.join("minecraft").join("client.jar"),
            forge_installer: Some(root.join("наш").join("cache").join("installer.jar")),
        };

        let on_change = silent();
        let cancelled = never();
        let progress = Progress::new(&on_change, &cancelled);

        copy_instance(&root, &scanned, &targets, &progress).await.unwrap();

        assert_eq!(
            std::fs::read_to_string(targets.minecraft.join("saves").join("Мир").join("level.dat")).unwrap(),
            "мир"
        );
        assert_eq!(std::fs::read_to_string(&targets.client_jar).unwrap(), "client");
        assert_eq!(
            std::fs::read_to_string(targets.forge_installer.as_ref().unwrap()).unwrap(),
            "installer"
        );

        std::fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn a_second_import_over_an_existing_instance_changes_nothing() {
        let root = prism_tree();
        let to = root.join("наш").join("instances").join("abc");

        let scanned = scan(&root).await.unwrap().pop().unwrap();

        let targets = InstanceTargets {
            minecraft: to.join("minecraft"),
            client_jar: to.join("minecraft").join("client.jar"),
            forge_installer: None,
        };

        let on_change = silent();
        let cancelled = never();

        let first = Progress::new(&on_change, &cancelled);
        copy_instance(&root, &scanned, &targets, &first).await.unwrap();

        // Пользователь поменял настройки уже в нашей сборке — перенос их не затрёт.
        std::fs::write(targets.minecraft.join("options.txt"), "fov:110").unwrap();

        let second = Progress::new(&on_change, &cancelled);
        copy_instance(&root, &scanned, &targets, &second).await.unwrap();

        assert_eq!(
            std::fs::read_to_string(targets.minecraft.join("options.txt")).unwrap(),
            "fov:110"
        );
        assert_eq!(second.stats().files, 0);
        assert_eq!(second.stats().skipped, first.stats().files);

        std::fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn a_vanilla_instance_needs_no_forge_installer() {
        let root = prism_tree();
        let to = root.join("наш");

        let mut scanned = scan(&root).await.unwrap().pop().unwrap();
        scanned.loader = Some(LoaderType::Vanilla);
        scanned.loader_version = None;

        let targets = InstanceTargets {
            minecraft: to.join("minecraft"),
            client_jar: to.join("client.jar"),
            forge_installer: Some(to.join("cache").join("installer.jar")),
        };

        let on_change = silent();
        let cancelled = never();
        let progress = Progress::new(&on_change, &cancelled);

        copy_instance(&root, &scanned, &targets, &progress).await.unwrap();

        assert!(!targets.forge_installer.unwrap().exists());

        std::fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn scanning_a_folder_without_instances_is_an_error() {
        let root = std::env::temp_dir().join(format!("cast-prism-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();

        assert!(scan(&root).await.is_err());

        std::fs::remove_dir_all(&root).ok();
    }
}
