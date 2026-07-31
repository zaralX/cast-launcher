//!
//! ```text
//! settings_dir/app.db
//! config_dir/profiles/<path>/   <- сама сборка, это сразу .minecraft
//! config_dir/meta/libraries/
//! config_dir/meta/assets/{indexes,objects}
//! config_dir/meta/java_versions/
//! config_dir/meta/versions/<id>/<id>.jar
//! config_dir/icons/
//! ```
//!

pub mod db;

use std::path::{Path, PathBuf};

use crate::config::JavaMode;
use crate::error::{CommandError, CommandResult};
use crate::instance::{InstanceSettings, LoaderType, Playtime};
use crate::meta::neoforge;

use super::copy::{self, Progress};
use super::{prism, ImportOptions, InstanceTargets, ManagedPack, ScannedInstance, SharedTargets};

pub const APP_DIR: &str = "ModrinthApp";

pub const PROFILES: &str = "profiles";
pub const META: &str = "meta";
pub const ICONS: &str = "icons";
pub const LIBRARIES: &str = "libraries";
pub const ASSETS: &str = "assets";
pub const VERSIONS: &str = "versions";
pub const JAVA: &str = "java_versions";

const LOADERS: &[(&str, &str, Option<LoaderType>)] = &[
    ("vanilla", "Vanilla", Some(LoaderType::Vanilla)),
    ("fabric", "Fabric", Some(LoaderType::Fabric)),
    ("forge", "Forge", Some(LoaderType::Forge)),
    ("neoforge", "NeoForge", Some(LoaderType::NeoForge)),
    ("quilt", "Quilt", None),
];

#[derive(Debug, Clone)]
pub struct Root {
    pub settings: PathBuf,
    pub config: PathBuf,
    instances: Vec<db::InstanceRow>,
}

impl Root {
    pub fn instances(&self) -> usize {
        self.instances.len()
    }

    pub fn profiles(&self) -> PathBuf {
        self.config.join(PROFILES)
    }

    pub fn meta(&self) -> PathBuf {
        self.config.join(META)
    }

    pub fn libraries(&self) -> PathBuf {
        self.meta().join(LIBRARIES)
    }

    pub fn assets(&self) -> PathBuf {
        self.meta().join(ASSETS)
    }

    pub fn versions(&self) -> PathBuf {
        self.meta().join(VERSIONS)
    }

    pub fn java_runtimes(&self) -> PathBuf {
        self.meta().join(JAVA)
    }

    pub fn icons(&self) -> PathBuf {
        self.config.join(ICONS)
    }
}

pub fn data_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    if let Ok(configured) = std::env::var("THESEUS_CONFIG_DIR") {
        if !configured.trim().is_empty() {
            dirs.push(PathBuf::from(configured));
        }
    }

    if cfg!(target_os = "windows") {
        if let Ok(appdata) = std::env::var("APPDATA") {
            dirs.push(PathBuf::from(appdata).join(APP_DIR));
        }
    } else if cfg!(target_os = "macos") {
        if let Ok(home) = std::env::var("HOME") {
            dirs.push(
                PathBuf::from(home)
                    .join("Library")
                    .join("Application Support")
                    .join(APP_DIR),
            );
        }
    } else {
        if let Ok(data_home) = std::env::var("XDG_DATA_HOME") {
            dirs.push(PathBuf::from(data_home).join(APP_DIR));
        }

        if let Ok(home) = std::env::var("HOME") {
            let home = PathBuf::from(home);

            dirs.push(home.join(".local").join("share").join(APP_DIR));
            dirs.push(
                home.join(".var")
                    .join("app")
                    .join("com.modrinth.ModrinthApp")
                    .join("data")
                    .join(APP_DIR),
            );
        }
    }

    dirs
}

pub fn is_data_dir(dir: &Path) -> bool {
    dir.join(db::DB_FILE).is_file()
}

pub fn detect() -> Option<PathBuf> {
    data_dirs().into_iter().find(|dir| is_data_dir(dir))
}

pub fn normalize(dir: &Path) -> PathBuf {
    if is_data_dir(dir) {
        return dir.to_path_buf();
    }

    if dir.file_name().is_some_and(|name| name == PROFILES) {
        if let Some(parent) = dir.parent() {
            if is_data_dir(parent) {
                return parent.to_path_buf();
            }
        }
    }

    let nested = dir.join(APP_DIR);

    if is_data_dir(&nested) {
        return nested;
    }

    dir.to_path_buf()
}

pub async fn open(dir: &Path) -> CommandResult<Root> {
    let settings = normalize(dir);

    if !is_data_dir(&settings) {
        return Err(CommandError::fs(format!(
            "Это не каталог данных Modrinth App: внутри нет {} ({})",
            db::DB_FILE,
            settings.display()
        )));
    }

    let snapshot = snapshot(&settings).await?;

    Ok(Root {
        config: snapshot.custom_dir.unwrap_or_else(|| settings.clone()),
        settings,
        instances: snapshot.instances,
    })
}

async fn snapshot(settings: &Path) -> CommandResult<db::Snapshot> {
    let settings = settings.to_path_buf();

    tokio::task::spawn_blocking(move || db::read(&settings))
        .await
        .map_err(|e| CommandError::task_panicked("чтение базы Modrinth App", e))?
}

pub async fn scan(root: &Root) -> Vec<ScannedInstance> {
    let forge_versions = prism::forge_versions(&root.libraries()).await;

    let mut found = Vec::new();

    for row in &root.instances {
        let mut scanned = parse(row);

        if scanned.blocked.is_none() && !root.profiles().join(&scanned.folder).is_dir() {
            scanned.blocked = Some("папки сборки нет на диске".into());
        }

        if scanned.is_importable() {
            if let Some(source) = icon_source(root, row.icon_path.as_deref()) {
                if let Some(name) = icon_name(&scanned.folder, &source) {
                    scanned.icon = Some(name);
                    scanned.icon_source = Some(source);
                }
            }
        }

        if scanned.loader == Some(LoaderType::Forge) {
            scanned.loader_version = scanned
                .loader_version
                .map(|version| prism::pick_forge_version(&forge_versions, &version));
        }

        found.push(scanned);
    }

    found
}

pub fn parse(row: &db::InstanceRow) -> ScannedInstance {
    let name = match row.name.trim() {
        "" => row.path.clone(),
        name => name.to_string(),
    };

    let mut scanned = ScannedInstance {
        settings: settings(row),
        playtime: playtime(row),
        pack: managed_pack(row),
        ..ScannedInstance::new(row.path.clone(), name)
    };

    if !is_folder_name(&row.path) {
        scanned.blocked = Some("в базе Modrinth App испорчен путь к папке сборки".into());
        return scanned;
    }

    let minecraft = row.game_version.trim().to_string();

    if minecraft.is_empty() {
        scanned.blocked = Some("в базе Modrinth App нет версии Minecraft".into());
        return scanned;
    }

    scanned.minecraft_version = minecraft.clone();

    let loader = row.loader.trim().to_ascii_lowercase();

    if loader.is_empty() || loader == "vanilla" {
        scanned.loader = Some(LoaderType::Vanilla);
        return scanned;
    }

    let Some((label, kind)) = LOADERS
        .iter()
        .find(|(key, _, _)| *key == loader)
        .map(|(_, label, kind)| (*label, *kind))
    else {
        scanned.blocked = Some(format!("лаунчер пока не умеет {}", row.loader.trim()));
        return scanned;
    };

    let version = row
        .loader_version
        .as_deref()
        .map(str::trim)
        .filter(|version| !version.is_empty());

    scanned.loader_label = match version {
        Some(version) => format!("{label} {version}"),
        None => label.to_string(),
    };

    let Some(kind) = kind else {
        scanned.blocked = Some(format!("лаунчер пока не умеет {label}"));
        return scanned;
    };

    let Some(version) = version else {
        scanned.blocked = Some(format!("в базе Modrinth App нет версии {label}"));
        return scanned;
    };

    scanned.loader = Some(kind);
    scanned.loader_version = Some(match kind {
        LoaderType::Forge => prism::forge_maven_version(&minecraft, version),
        LoaderType::NeoForge => neoforge::maven_version(&minecraft, version),
        _ => version.to_string(),
    });

    scanned
}

fn is_folder_name(path: &str) -> bool {
    !path.trim().is_empty()
        && path != "."
        && path != ".."
        && Path::new(path).file_name().is_some_and(|name| name == path)
}

fn settings(row: &db::InstanceRow) -> InstanceSettings {
    let java_path = row.java_path.clone().unwrap_or_default();
    let override_java = !java_path.is_empty();

    InstanceSettings {
        override_memory: row.memory_max.is_some(),
        min_ram: 0,
        max_ram: row.memory_max.unwrap_or(0),
        override_java,
        java_mode: if override_java { JavaMode::Manual } else { JavaMode::default() },
        java_path,
    }
}

fn playtime(row: &db::InstanceRow) -> Playtime {
    Playtime {
        total_seconds: row.time_played,
        last_seconds: 0,
        last_played_at: row
            .last_played
            .filter(|seconds| *seconds > 0)
            .map(|seconds| seconds as u64 * 1000)
            .unwrap_or(0),
    }
}

fn managed_pack(row: &db::InstanceRow) -> Option<ManagedPack> {
    Some(ManagedPack {
        provider: "modrinth".into(),
        project_id: row.project_id.clone()?,
        version_id: row.version_id.clone()?,
        version_name: String::new(),
        name: String::new(),
    })
}

pub fn icon_source(root: &Root, icon_path: Option<&str>) -> Option<PathBuf> {
    let icon_path = icon_path.map(str::trim).filter(|path| !path.is_empty())?;
    let raw = Path::new(icon_path);

    let candidates = if raw.is_absolute() {
        vec![raw.to_path_buf()]
    } else {
        vec![
            root.config.join(raw),
            root.settings.join(raw),
            root.icons().join(raw),
        ]
    };

    candidates.into_iter().find(|path| path.is_file())
}

fn icon_name(folder: &str, source: &Path) -> Option<String> {
    crate::icons::mime(source)?;

    let extension = source.extension()?.to_string_lossy().to_ascii_lowercase();

    Some(format!("modrinth_{folder}.{extension}"))
}

pub async fn client_jar(root: &Root, minecraft_version: &str) -> Option<PathBuf> {
    let jar = |version: &str| root.versions().join(version).join(format!("{version}.jar"));

    let exact = jar(minecraft_version);

    if exact.is_file() {
        return Some(exact);
    }

    let prefix = format!("{minecraft_version}-");
    let mut entries = tokio::fs::read_dir(root.versions()).await.ok()?;

    while let Ok(Some(entry)) = entries.next_entry().await {
        let name = entry.file_name().to_string_lossy().to_string();

        if !name.starts_with(&prefix) {
            continue;
        }

        let merged = jar(&name);

        if merged.is_file() {
            return Some(merged);
        }
    }

    None
}

pub async fn copy_shared(
    root: &Root,
    options: &ImportOptions,
    targets: &SharedTargets,
    progress: &Progress<'_>,
    on_step: impl Fn(&str),
) -> CommandResult<()> {
    if options.libraries {
        on_step("Библиотеки");
        copy::merge_dir(&root.libraries(), &targets.libraries, progress).await?;
    }

    if options.assets {
        let assets = root.assets();

        on_step("Ресурсы игры");
        copy::merge_dir(&assets.join("indexes"), &targets.asset_indexes, progress).await?;
        copy::merge_dir(&assets.join("objects"), &targets.asset_objects, progress).await?;
    }

    if options.java {
        on_step("Java");
        copy::merge_dir(&root.java_runtimes(), &targets.java_runtimes, progress).await?;
    }

    Ok(())
}

pub async fn copy_instance(
    root: &Root,
    scanned: &ScannedInstance,
    targets: &InstanceTargets,
    progress: &Progress<'_>,
) -> CommandResult<()> {
    let source = root.profiles().join(&scanned.folder);

    copy::merge_dir(&source, &targets.minecraft, progress).await?;

    if let Some(jar) = client_jar(root, &scanned.minecraft_version).await {
        copy::copy_file(&jar, &targets.client_jar, progress).await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row() -> db::InstanceRow {
        db::InstanceRow {
            path: "Fabulously Optimized".into(),
            name: "Fabulously Optimized 12.0.5".into(),
            icon_path: None,
            game_version: "1.21.1".into(),
            loader: "fabric".into(),
            loader_version: Some("0.16.5".into()),
            project_id: Some("1KVo5zza".into()),
            version_id: Some("fzpQA5K4".into()),
            java_path: None,
            memory_max: None,
            last_played: None,
            time_played: 0,
        }
    }

    fn scratch() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("cast-modrinth-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();

        dir
    }

    fn root_at(dir: &Path) -> Root {
        Root {
            settings: dir.to_path_buf(),
            config: dir.to_path_buf(),
            instances: Vec::new(),
        }
    }

    #[test]
    fn a_fabric_instance_is_mapped_field_by_field() {
        let scanned = parse(&row());

        assert!(scanned.is_importable());
        assert_eq!(scanned.folder, "Fabulously Optimized");
        assert_eq!(scanned.name, "Fabulously Optimized 12.0.5");
        assert_eq!(scanned.minecraft_version, "1.21.1");
        assert_eq!(scanned.loader, Some(LoaderType::Fabric));
        assert_eq!(scanned.loader_version.as_deref(), Some("0.16.5"));
        assert_eq!(scanned.loader_label, "Fabric 0.16.5");
    }

    #[test]
    fn a_linked_modpack_is_carried_over_as_a_modrinth_one() {
        let pack = parse(&row()).pack.unwrap();

        assert_eq!(pack.provider, "modrinth");
        assert_eq!(pack.project_id, "1KVo5zza");
        assert_eq!(pack.version_id, "fzpQA5K4");
    }

    #[test]
    fn an_instance_linked_to_a_project_without_a_version_is_not_a_modpack() {
        let unlinked = db::InstanceRow {
            version_id: None,
            ..row()
        };

        assert!(parse(&unlinked).pack.is_none());
    }

    #[test]
    fn forge_versions_gain_the_minecraft_prefix_modrinth_leaves_out() {
        let scanned = parse(&db::InstanceRow {
            game_version: "1.20.1".into(),
            loader: "forge".into(),
            loader_version: Some("47.4.13".into()),
            ..row()
        });

        assert_eq!(scanned.loader, Some(LoaderType::Forge));
        assert_eq!(scanned.loader_version.as_deref(), Some("1.20.1-47.4.13"));
        assert_eq!(scanned.loader_label, "Forge 47.4.13");
    }

    #[test]
    fn neoforge_for_1_20_1_gains_the_game_version_too() {
        let scanned = parse(&db::InstanceRow {
            game_version: "1.20.1".into(),
            loader: "neoforge".into(),
            loader_version: Some("47.1.106".into()),
            ..row()
        });

        assert_eq!(scanned.loader_version.as_deref(), Some("1.20.1-47.1.106"));

        let modern = parse(&db::InstanceRow {
            game_version: "1.21.1".into(),
            loader: "neoforge".into(),
            loader_version: Some("21.1.213".into()),
            ..row()
        });

        assert_eq!(modern.loader_version.as_deref(), Some("21.1.213"));
    }

    #[test]
    fn an_instance_without_a_loader_is_vanilla() {
        for loader in ["vanilla", "", "  "] {
            let scanned = parse(&db::InstanceRow {
                loader: loader.into(),
                loader_version: None,
                ..row()
            });

            assert_eq!(scanned.loader, Some(LoaderType::Vanilla), "loader={loader:?}");
            assert_eq!(scanned.loader_label, "Vanilla");
            assert!(scanned.loader_version.is_none());
            assert!(scanned.is_importable());
        }
    }

    #[test]
    fn quilt_is_reported_rather_than_silently_dropped() {
        let scanned = parse(&db::InstanceRow {
            loader: "quilt".into(),
            loader_version: Some("0.28.1".into()),
            ..row()
        });

        assert_eq!(scanned.loader_label, "Quilt 0.28.1");
        assert!(scanned.blocked.unwrap().contains("Quilt"));
    }

    #[test]
    fn an_unknown_loader_names_itself_in_the_reason() {
        let scanned = parse(&db::InstanceRow {
            loader: "liteloader".into(),
            ..row()
        });

        assert!(scanned.blocked.unwrap().contains("liteloader"));
    }

    #[test]
    fn a_loader_without_a_version_cannot_be_installed() {
        let scanned = parse(&db::InstanceRow {
            loader_version: None,
            ..row()
        });

        assert_eq!(scanned.loader_label, "Fabric");
        assert!(scanned.blocked.unwrap().contains("Fabric"));
    }

    #[test]
    fn an_instance_without_a_game_version_is_blocked() {
        let scanned = parse(&db::InstanceRow {
            game_version: String::new(),
            ..row()
        });

        assert!(scanned.blocked.unwrap().contains("Minecraft"));
    }

    #[test]
    fn a_nameless_instance_falls_back_to_its_folder() {
        let scanned = parse(&db::InstanceRow {
            name: "   ".into(),
            ..row()
        });

        assert_eq!(scanned.name, "Fabulously Optimized");
    }

    #[test]
    fn only_the_maximum_memory_comes_over_because_that_is_all_there_is() {
        let settings = parse(&db::InstanceRow {
            memory_max: Some(6144),
            ..row()
        })
        .settings;

        assert!(settings.override_memory);
        assert_eq!(settings.max_ram, 6144);
        assert_eq!(settings.min_ram, 0, "нижнюю границу берём из общих настроек");
    }

    #[test]
    fn an_untouched_instance_overrides_nothing() {
        assert!(!parse(&row()).settings.overrides_anything());
    }

    #[test]
    fn a_java_chosen_by_hand_becomes_a_manual_override() {
        let settings = parse(&db::InstanceRow {
            java_path: Some("C:/jdk21/bin/javaw.exe".into()),
            ..row()
        })
        .settings;

        assert!(settings.override_java);
        assert_eq!(settings.java_mode, JavaMode::Manual);
        assert_eq!(settings.java_path, "C:/jdk21/bin/javaw.exe");
    }

    #[test]
    fn playtime_arrives_as_the_sum_of_both_counters_in_milliseconds() {
        let playtime = parse(&db::InstanceRow {
            last_played: Some(1_761_212_747),
            time_played: 705_341,
            ..row()
        })
        .playtime;

        assert_eq!(playtime.total_seconds, 705_341);
        assert_eq!(playtime.last_played_at, 1_761_212_747_000);
        assert_eq!(playtime.last_seconds, 0);
    }

    #[test]
    fn an_instance_never_launched_arrives_with_a_zeroed_counter() {
        assert_eq!(parse(&row()).playtime, Playtime::default());
    }

    #[test]
    fn conversion_produces_an_uninstalled_instance() {
        let instance = parse(&row()).to_instance("abc".into(), "fo.png".into()).unwrap();

        assert_eq!(instance.id, "abc");
        assert_eq!(instance.loader, LoaderType::Fabric);
        assert_eq!(instance.minecraft_version, "1.21.1");
        assert!(!instance.installed);
        assert!(instance.pack.is_none(), "пак проставляется отдельно");
    }

    #[test]
    fn a_data_dir_is_recognised_by_its_database() {
        let root = scratch();
        std::fs::write(root.join(db::DB_FILE), b"sqlite").unwrap();
        std::fs::create_dir_all(root.join(PROFILES)).unwrap();

        assert!(is_data_dir(&root));
        assert_eq!(normalize(&root), root);
        assert_eq!(normalize(&root.join(PROFILES)), root);
        assert!(!is_data_dir(&root.join("нет")));

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn pointing_at_the_parent_folder_still_finds_the_app() {
        let root = scratch();
        let app = root.join(APP_DIR);
        std::fs::create_dir_all(&app).unwrap();
        std::fs::write(app.join(db::DB_FILE), b"sqlite").unwrap();

        assert_eq!(normalize(&root), app);

        std::fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn the_vanilla_client_is_found_under_its_own_name() {
        let root = scratch();
        let versions = root.join(META).join(VERSIONS).join("1.21.1");
        std::fs::create_dir_all(&versions).unwrap();
        std::fs::write(versions.join("1.21.1.jar"), "client").unwrap();

        let found = client_jar(&root_at(&root), "1.21.1").await.unwrap();
        assert_eq!(std::fs::read_to_string(found).unwrap(), "client");

        std::fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn a_modded_client_is_found_in_the_merged_version_folder() {
        let root = scratch();
        let versions = root.join(META).join(VERSIONS).join("1.20.1-47.4.13");
        std::fs::create_dir_all(&versions).unwrap();
        std::fs::write(versions.join("1.20.1-47.4.13.jar"), "client").unwrap();

        let found = client_jar(&root_at(&root), "1.20.1").await.unwrap();
        assert_eq!(std::fs::read_to_string(found).unwrap(), "client");

        assert!(client_jar(&root_at(&root), "1.20").await.is_none(), "префикс точный");
        assert!(client_jar(&root_at(&root), "1.21.1").await.is_none());

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn an_icon_is_looked_for_next_to_both_roots() {
        let root = scratch();
        let icons = root.join(ICONS);
        std::fs::create_dir_all(&icons).unwrap();
        std::fs::write(icons.join("fo.png"), b"pixels").unwrap();

        let at = root_at(&root);

        assert_eq!(
            icon_source(&at, Some("icons/fo.png")),
            Some(icons.join("fo.png"))
        );
        assert_eq!(icon_source(&at, Some("fo.png")), Some(icons.join("fo.png")));
        assert_eq!(
            icon_source(&at, Some(icons.join("fo.png").to_str().unwrap())),
            Some(icons.join("fo.png"))
        );
        assert!(icon_source(&at, Some("нет.png")).is_none());
        assert!(icon_source(&at, Some("  ")).is_none());
        assert!(icon_source(&at, None).is_none());

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn an_icon_is_named_after_the_instance_so_two_of_them_never_collide() {
        let first = icon_name("Create Azure", Path::new("/icons/icon.png"));
        let second = icon_name("Fabulously Optimized", Path::new("/other/icon.png"));

        assert_eq!(first.as_deref(), Some("modrinth_Create Azure.png"));
        assert_ne!(first, second, "иначе вторая сборка возьмёт картинку первой");
    }

    #[test]
    fn an_icon_that_is_not_a_picture_is_ignored() {
        assert!(icon_name("x", Path::new("/icons/readme.txt")).is_none());
        assert!(icon_name("x", Path::new("/icons/noext")).is_none());
    }

    #[test]
    fn every_icon_name_we_make_up_is_one_our_own_folder_accepts() {
        for folder in ["Create Azure", "1.20.1", "Мой пак", "a.b.c"] {
            let name = icon_name(folder, Path::new("/icons/icon.WEBP")).unwrap();

            assert!(
                crate::icons::resolve(Path::new("/icons"), &name).is_ok(),
                "имя {name:?} не принимается"
            );
        }
    }

    #[test]
    fn a_broken_folder_path_is_blocked_before_it_can_swallow_the_whole_tree() {
        for path in ["", "   ", ".", "..", "../соседняя", "вложенная/папка", "C:\\другое"] {
            let scanned = parse(&db::InstanceRow {
                path: path.into(),
                ..row()
            });

            assert!(scanned.blocked.is_some(), "путь {path:?} должен блокироваться");
        }

        assert!(is_folder_name("Create Azure"));
        assert!(is_folder_name("1.20.1"));
    }

    fn modrinth_tree() -> PathBuf {
        let root = scratch();

        let saves = root.join(PROFILES).join("Create Azure").join("saves").join("Мир");
        std::fs::create_dir_all(&saves).unwrap();
        std::fs::write(saves.join("level.dat"), "мир").unwrap();
        std::fs::write(
            root.join(PROFILES).join("Create Azure").join("options.txt"),
            "fov:80",
        )
        .unwrap();

        let versions = root.join(META).join(VERSIONS).join("1.21.1");
        std::fs::create_dir_all(&versions).unwrap();
        std::fs::write(versions.join("1.21.1.jar"), "client").unwrap();

        let libraries = root.join(META).join(LIBRARIES).join("net").join("fabricmc");
        std::fs::create_dir_all(&libraries).unwrap();
        std::fs::write(libraries.join("fabric.jar"), "lib").unwrap();

        let objects = root.join(META).join(ASSETS).join("objects").join("ab");
        std::fs::create_dir_all(&objects).unwrap();
        std::fs::create_dir_all(root.join(META).join(ASSETS).join("indexes")).unwrap();
        std::fs::write(objects.join("abcdef"), "звук").unwrap();
        std::fs::write(
            root.join(META).join(ASSETS).join("indexes").join("17.json"),
            "{}",
        )
        .unwrap();

        let java = root.join(META).join(JAVA).join("zulu21").join("bin");
        std::fs::create_dir_all(&java).unwrap();
        std::fs::write(java.join("javaw.exe"), "java").unwrap();

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
        let root = modrinth_tree();
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
        copy_shared(
            &root_at(&root),
            &ImportOptions::default(),
            &targets,
            &progress,
            |step| steps.lock().unwrap().push(step.to_string()),
        )
        .await
        .unwrap();

        assert_eq!(*steps.lock().unwrap(), vec!["Библиотеки", "Ресурсы игры", "Java"]);
        assert!(targets.libraries.join("net").join("fabricmc").join("fabric.jar").is_file());
        assert!(targets.asset_objects.join("ab").join("abcdef").is_file());
        assert!(targets.asset_indexes.join("17.json").is_file());
        assert!(targets.java_runtimes.join("zulu21").join("bin").join("javaw.exe").is_file());

        std::fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn switched_off_folders_are_left_behind() {
        let root = modrinth_tree();
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

        copy_shared(&root_at(&root), &options, &targets, &progress, |_| {})
            .await
            .unwrap();

        assert!(targets.libraries.is_dir());
        assert!(!targets.asset_objects.exists());
        assert!(!targets.java_runtimes.exists());

        std::fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn an_instance_arrives_with_its_world_and_client() {
        let root = modrinth_tree();
        let to = root.join("наш").join("instances").join("abc");

        let scanned = parse(&db::InstanceRow {
            path: "Create Azure".into(),
            name: "Create Azure".into(),
            ..row()
        });

        let targets = InstanceTargets {
            minecraft: to.join("minecraft"),
            client_jar: to.join("minecraft").join("client.jar"),
            loader_installer: None,
        };

        let on_change = silent();
        let cancelled = never();
        let progress = Progress::new(&on_change, &cancelled);

        copy_instance(&root_at(&root), &scanned, &targets, &progress)
            .await
            .unwrap();

        assert_eq!(
            std::fs::read_to_string(targets.minecraft.join("saves").join("Мир").join("level.dat"))
                .unwrap(),
            "мир"
        );
        assert_eq!(std::fs::read_to_string(&targets.client_jar).unwrap(), "client");

        std::fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn a_second_import_over_an_existing_instance_changes_nothing() {
        let root = modrinth_tree();
        let to = root.join("наш").join("instances").join("abc");

        let scanned = parse(&db::InstanceRow {
            path: "Create Azure".into(),
            ..row()
        });

        let targets = InstanceTargets {
            minecraft: to.join("minecraft"),
            client_jar: to.join("minecraft").join("client.jar"),
            loader_installer: None,
        };

        let on_change = silent();
        let cancelled = never();

        let first = Progress::new(&on_change, &cancelled);
        copy_instance(&root_at(&root), &scanned, &targets, &first).await.unwrap();

        std::fs::write(targets.minecraft.join("options.txt"), "fov:110").unwrap();

        let second = Progress::new(&on_change, &cancelled);
        copy_instance(&root_at(&root), &scanned, &targets, &second).await.unwrap();

        assert_eq!(
            std::fs::read_to_string(targets.minecraft.join("options.txt")).unwrap(),
            "fov:110"
        );
        assert_eq!(second.stats().files, 0);
        assert_eq!(second.stats().skipped, first.stats().files);

        std::fs::remove_dir_all(&root).ok();
    }

    fn database_at(root: &Path, schema: db::Schema) {
        let connection = rusqlite::Connection::open(root.join(db::DB_FILE)).unwrap();

        connection
            .execute_batch(
                "CREATE TABLE settings (id INTEGER PRIMARY KEY, custom_dir TEXT NULL);
                 INSERT INTO settings VALUES (0, NULL);",
            )
            .unwrap();

        let sql = match schema {
            db::Schema::Profiles => {
                "CREATE TABLE profiles (
                    path TEXT PRIMARY KEY, name TEXT, icon_path TEXT,
                    game_version TEXT, mod_loader TEXT, mod_loader_version TEXT,
                    linked_project_id TEXT, linked_version_id TEXT,
                    override_java_path TEXT, override_mc_memory_max INTEGER,
                    last_played INTEGER,
                    submitted_time_played INTEGER DEFAULT 0,
                    recent_time_played INTEGER DEFAULT 0
                 );
                 INSERT INTO profiles VALUES (
                    'Create Azure', 'Create Azure', 'icons/azure.png',
                    '1.21.1', 'fabric', '0.16.5', NULL, NULL, NULL, NULL, NULL, 0, 0
                 );
                 INSERT INTO profiles VALUES (
                    'Ушедшая', 'Ушедшая', NULL,
                    '1.21.1', 'fabric', '0.16.5', NULL, NULL, NULL, NULL, NULL, 0, 0
                 );"
            }
            db::Schema::Instances => {
                "CREATE TABLE instances (
                    id TEXT PRIMARY KEY, path TEXT, applied_content_set_id TEXT,
                    name TEXT, icon_path TEXT, last_played INTEGER,
                    submitted_time_played INTEGER DEFAULT 0,
                    recent_time_played INTEGER DEFAULT 0
                 );
                 CREATE TABLE instance_content_sets (
                    id TEXT PRIMARY KEY, instance_id TEXT, created INTEGER,
                    game_version TEXT, loader TEXT, loader_version TEXT
                 );
                 CREATE TABLE instance_links (
                    instance_id TEXT PRIMARY KEY,
                    modrinth_project_id TEXT, modrinth_version_id TEXT
                 );
                 CREATE TABLE instance_launch_overrides (
                    instance_id TEXT PRIMARY KEY, overrides BLOB
                 );
                 INSERT INTO instances VALUES (
                    'i1', 'Create Azure', 's1', 'Create Azure', 'icons/azure.png', NULL, 0, 0
                 );
                 INSERT INTO instance_content_sets VALUES (
                    's1', 'i1', 1, '1.21.1', 'fabric', '0.16.5'
                 );
                 -- У этой сборки набор контента не помечен применённым: берём единственный.
                 INSERT INTO instances VALUES (
                    'i2', 'Ушедшая', NULL, 'Ушедшая', NULL, NULL, 0, 0
                 );
                 INSERT INTO instance_content_sets VALUES (
                    's2', 'i2', 1, '1.21.1', 'fabric', '0.16.5'
                 );"
            }
        };

        connection.execute_batch(sql).unwrap();
        drop(connection);
    }

    #[tokio::test]
    async fn scanning_reads_the_database_and_checks_it_against_the_disk() {
        for schema in [db::Schema::Instances, db::Schema::Profiles] {
            scanning_works_on(schema).await;
        }
    }

    async fn scanning_works_on(schema: db::Schema) {
        let root = modrinth_tree();
        database_at(&root, schema);

        let icons = root.join(ICONS);
        std::fs::create_dir_all(&icons).unwrap();
        std::fs::write(icons.join("azure.png"), b"pixels").unwrap();

        let opened = open(&root).await.unwrap();
        assert_eq!(opened.config, root, "custom_dir не задан - всё лежит рядом");
        assert_eq!(opened.instances(), 2, "база прочитана один раз, при открытии");

        let found = scan(&opened).await;
        assert_eq!(found.len(), 2);

        let azure = found.iter().find(|i| i.folder == "Create Azure").unwrap();
        assert!(azure.is_importable(), "{schema:?}");
        assert_eq!(azure.loader, Some(LoaderType::Fabric), "{schema:?}");
        assert_eq!(azure.icon.as_deref(), Some("modrinth_Create Azure.png"));
        assert_eq!(azure.icon_source.as_deref(), Some(icons.join("azure.png").as_path()));

        let gone = found.iter().find(|i| i.folder == "Ушедшая").unwrap();
        assert_eq!(gone.blocked.as_deref(), Some("папки сборки нет на диске"), "{schema:?}");
        assert!(gone.icon.is_none(), "у непереносимой сборки иконку не ищем");

        std::fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn a_custom_directory_sends_us_looking_elsewhere() {
        let settings = scratch();
        let config = modrinth_tree();

        database_at(&settings, db::Schema::Instances);

        let connection = rusqlite::Connection::open(settings.join(db::DB_FILE)).unwrap();
        connection
            .execute("UPDATE settings SET custom_dir = ?1", [config.to_str().unwrap()])
            .unwrap();
        drop(connection);

        let opened = open(&settings).await.unwrap();

        assert_eq!(opened.settings, settings);
        assert_eq!(opened.config, config);
        assert!(opened.profiles().join("Create Azure").is_dir());

        let found = scan(&opened).await;
        assert!(found.iter().find(|i| i.folder == "Create Azure").unwrap().is_importable());

        std::fs::remove_dir_all(&settings).ok();
        std::fs::remove_dir_all(&config).ok();
    }

    #[tokio::test]
    async fn opening_a_folder_without_a_database_is_an_error() {
        let root = scratch();

        assert!(open(&root).await.unwrap_err().message.contains(db::DB_FILE));

        std::fs::remove_dir_all(&root).ok();
    }
}
