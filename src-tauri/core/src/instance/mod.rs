use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::error::{CommandError, CommandResult};
use crate::fs_util::{read_json_opt, write_json_atomic};
use crate::paths::LauncherPaths;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LoaderType {
    Vanilla,
    Fabric,
    Forge,
}

impl LoaderType {
    pub fn label(self) -> &'static str {
        match self {
            Self::Vanilla => "Vanilla",
            Self::Fabric => "Fabric",
            Self::Forge => "Forge",
        }
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
    #[serde(skip)]
    pub dir: String,
}

fn default_version() -> u32 {
    1
}

impl Instance {
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
