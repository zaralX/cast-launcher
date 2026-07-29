use std::path::{Path, PathBuf};

use serde::Serialize;

///
/// ```text
/// <config_root>/                  каталог конфигурации приложения
///   config.json
///   accounts.json
///   instances/<id>/instance.json
///                 /minecraft/{client.jar, natives/, ...}
/// <root>/                         "Файлы лаунчера" из настроек, по умолчанию = config_root
///   libraries/<maven path>
///   assets/indexes/<id>.json
///   assets/objects/<ab>/<hash>
///   cache/<loader>/<version>/{installer.jar, client.json, installed.json}
///   cache/meta/<hash>.json + .etag        кэш сетевых манифестов
///   runtime/<component>/                  рантаймы Java от Mojang
///   logs/<instance id>/<timestamp>.log    логи запусков
/// ```
#[derive(Debug, Clone)]
pub struct LauncherPaths {
    config_root: PathBuf,
    root: PathBuf,
}

impl LauncherPaths {
    pub fn new(config_root: PathBuf, launcher_dir: Option<&str>) -> Self {
        let root = launcher_dir
            .map(str::trim)
            .filter(|dir| !dir.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| config_root.clone());

        Self { config_root, root }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn config_root(&self) -> &Path {
        &self.config_root
    }

    pub fn config_file(&self) -> PathBuf {
        self.config_root.join("config.json")
    }

    pub fn accounts_file(&self) -> PathBuf {
        self.config_root.join("accounts.json")
    }

    pub fn instances_root(&self) -> PathBuf {
        self.config_root.join("instances")
    }

    pub fn icons(&self) -> PathBuf {
        self.config_root.join("icons")
    }

    pub fn instance(&self, id: &str) -> InstancePaths {
        InstancePaths::new(self.instances_root().join(id))
    }

    pub fn libraries(&self) -> PathBuf {
        self.root.join("libraries")
    }

    pub fn library(&self, relative: &str) -> PathBuf {
        join_relative(&self.libraries(), relative)
    }

    pub fn assets(&self) -> PathBuf {
        self.root.join("assets")
    }

    pub fn asset_indexes(&self) -> PathBuf {
        self.assets().join("indexes")
    }

    pub fn asset_index(&self, id: &str) -> PathBuf {
        self.asset_indexes().join(format!("{id}.json"))
    }

    pub fn asset_objects(&self) -> PathBuf {
        self.assets().join("objects")
    }

    pub fn asset_object(&self, hash: &str) -> PathBuf {
        self.asset_objects().join(&hash[..2.min(hash.len())]).join(hash)
    }

    pub fn cache(&self) -> PathBuf {
        self.root.join("cache")
    }

    pub fn meta_cache(&self) -> PathBuf {
        self.cache().join("meta")
    }

    pub fn loader_cache(&self, loader: &str, version: &str) -> LoaderPaths {
        LoaderPaths::new(self.cache().join(loader).join(version))
    }

    pub fn java_runtimes(&self) -> PathBuf {
        self.root.join("runtime")
    }

    pub fn logs(&self) -> PathBuf {
        self.root.join("logs")
    }

    pub fn instance_logs(&self, instance_id: &str) -> PathBuf {
        self.logs().join(instance_id)
    }

    pub fn scratch(&self, purpose: &str) -> PathBuf {
        self.cache()
            .join("tmp")
            .join(format!("{purpose}-{}", uuid::Uuid::new_v4().simple()))
    }
}

#[derive(Debug, Clone)]
pub struct InstancePaths {
    root: PathBuf,
}

impl InstancePaths {
    fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn config_file(&self) -> PathBuf {
        self.root.join("instance.json")
    }

    pub fn pack_files(&self) -> PathBuf {
        self.root.join("pack-files.json")
    }

    pub fn minecraft(&self) -> PathBuf {
        self.root.join("minecraft")
    }

    pub fn client_jar(&self) -> PathBuf {
        self.minecraft().join("client.jar")
    }

    pub fn natives(&self) -> PathBuf {
        self.minecraft().join("natives")
    }
}

#[derive(Debug, Clone)]
pub struct LoaderPaths {
    root: PathBuf,
}

impl LoaderPaths {
    fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn installer_jar(&self) -> PathBuf {
        self.root.join("installer.jar")
    }

    pub fn client_json(&self) -> PathBuf {
        self.root.join("client.json")
    }

    pub fn installed_json(&self) -> PathBuf {
        self.root.join("installed.json")
    }
}

fn join_relative(base: &Path, relative: &str) -> PathBuf {
    let mut path = base.to_path_buf();
    for part in relative.split('/').filter(|part| !part.is_empty() && *part != ".") {
        path.push(part);
    }
    path
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PathsSnapshot {
    pub root: String,
    pub config_root: String,
    pub instances_root: String,
    pub icons: String,
    pub libraries: String,
    pub assets: String,
    pub java_runtimes: String,
    pub logs: String,
}

impl From<&LauncherPaths> for PathsSnapshot {
    fn from(paths: &LauncherPaths) -> Self {
        Self {
            root: paths.root().display().to_string(),
            config_root: paths.config_root().display().to_string(),
            instances_root: paths.instances_root().display().to_string(),
            icons: paths.icons().display().to_string(),
            libraries: paths.libraries().display().to_string(),
            assets: paths.assets().display().to_string(),
            java_runtimes: paths.java_runtimes().display().to_string(),
            logs: paths.logs().display().to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paths() -> LauncherPaths {
        LauncherPaths::new(PathBuf::from("/cfg"), None)
    }

    #[test]
    fn launcher_dir_overrides_only_shared_files() {
        let custom = LauncherPaths::new(PathBuf::from("/cfg"), Some("/data"));

        assert_eq!(custom.libraries(), PathBuf::from("/data").join("libraries"));
        assert_eq!(custom.instances_root(), PathBuf::from("/cfg").join("instances"));
        assert_eq!(custom.config_file(), PathBuf::from("/cfg").join("config.json"));
    }

    #[test]
    fn empty_launcher_dir_falls_back_to_config_root() {
        let blank = LauncherPaths::new(PathBuf::from("/cfg"), Some("   "));
        assert_eq!(blank.root(), Path::new("/cfg"));
    }

    #[test]
    fn maven_paths_are_split_into_components() {
        let library = paths().library("org/lwjgl/lwjgl/3.3.1/lwjgl-3.3.1.jar");

        assert!(library.ends_with("lwjgl-3.3.1.jar"));
        assert_eq!(library.components().count(), paths().libraries().components().count() + 5);
    }

    #[test]
    fn asset_objects_are_sharded_by_hash_prefix() {
        let object = paths().asset_object("abcdef0123456789");
        assert!(object.ends_with(Path::new("ab").join("abcdef0123456789")));
    }

    #[test]
    fn scratch_dirs_are_unique() {
        assert_ne!(paths().scratch("forge"), paths().scratch("forge"));
    }

    #[test]
    fn every_loader_caches_its_installer_apart() {
        let forge = paths().loader_cache("forge", "1.20.1-47.4.13");
        let neoforge = paths().loader_cache("neoforge", "21.1.243");

        assert!(forge.installer_jar().ends_with(Path::new("forge/1.20.1-47.4.13/installer.jar")));
        assert!(neoforge.client_json().ends_with(Path::new("neoforge/21.1.243/client.json")));
        assert_ne!(forge.root(), neoforge.root());
    }
}
