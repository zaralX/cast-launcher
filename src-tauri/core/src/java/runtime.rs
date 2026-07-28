use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::error::{CommandError, CommandResult};
use crate::mojang::rules::{MojangOs, RuntimeContext};
use crate::net::download::{DownloadOptions, DownloadRegistry, DownloadTask, ProgressSink};
use crate::net::meta_cache::MetaCache;

const RUNTIME_INDEX_URL: &str =
    "https://launchermeta.mojang.com/v1/products/java-runtime/2ec0cc96c44e5a76b9c8b7c39df7210883d12871/all.json";

#[derive(Debug, Deserialize)]
struct RuntimeEntry {
    #[serde(default)]
    version: Option<RuntimeVersion>,
    #[serde(default)]
    manifest: Option<RuntimeManifestRef>,
}

#[derive(Debug, Deserialize)]
struct RuntimeVersion {
    #[serde(default)]
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RuntimeManifestRef {
    url: String,
}

#[derive(Debug, Deserialize)]
struct RuntimeManifest {
    #[serde(default)]
    files: HashMap<String, RuntimeFile>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeFile {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    executable: bool,
    #[serde(default)]
    target: Option<String>,
    #[serde(default)]
    downloads: Option<RuntimeDownloads>,
}

#[derive(Debug, Deserialize)]
struct RuntimeDownloads {
    #[serde(default)]
    raw: Option<RuntimeArtifact>,
}

#[derive(Debug, Deserialize)]
struct RuntimeArtifact {
    url: String,
    #[serde(default)]
    sha1: Option<String>,
    #[serde(default)]
    size: Option<u64>,
}

pub fn platform_key(ctx: &RuntimeContext) -> Option<&'static str> {
    match (ctx.os, ctx.arch.as_str()) {
        (MojangOs::Windows, "arm64") => Some("windows-arm64"),
        (MojangOs::Windows, "x86") => Some("windows-x86"),
        (MojangOs::Windows, _) => Some("windows-x64"),
        (MojangOs::Osx, "arm64") => Some("mac-os-arm64"),
        (MojangOs::Osx, _) => Some("mac-os"),
        (MojangOs::Linux, "x86") => Some("linux-i386"),
        (MojangOs::Linux, "x86_64") => Some("linux"),
        _ => None,
    }
}

#[cfg_attr(not(unix), allow(dead_code))]
struct RuntimeLink {
    path: String,
    target: String,
}

pub async fn install(
    component: &str,
    target_dir: &Path,
    ctx: &RuntimeContext,
    meta: &MetaCache,
    downloads: &DownloadRegistry,
    job_id: &str,
    on_progress: Option<ProgressSink>,
) -> CommandResult<Option<String>> {
    let Some(platform) = platform_key(ctx) else {
        return Ok(None);
    };

    let index: HashMap<String, HashMap<String, Vec<RuntimeEntry>>> =
        meta.fetch_json(RUNTIME_INDEX_URL).await?;

    let Some(entry) = index
        .get(platform)
        .and_then(|components| components.get(component))
        .and_then(|entries| entries.first())
    else {
        return Ok(None);
    };

    let Some(manifest_ref) = &entry.manifest else {
        return Ok(None);
    };

    let manifest: RuntimeManifest = meta.fetch_json(&manifest_ref.url).await?;

    let mut tasks = Vec::new();
    let mut executables = Vec::new();
    let mut links = Vec::new();

    for (relative, file) in &manifest.files {
        match file.kind.as_str() {
            "link" => {
                if let Some(target) = &file.target {
                    links.push(RuntimeLink {
                        path: relative.clone(),
                        target: target.clone(),
                    });
                }
            }
            "file" => {
                let Some(raw) = file.downloads.as_ref().and_then(|d| d.raw.as_ref()) else {
                    continue;
                };

                tasks.push(DownloadTask::verified(
                    raw.url.clone(),
                    join_relative(target_dir, relative),
                    raw.size,
                    raw.sha1.clone(),
                ));

                if file.executable {
                    executables.push(relative.clone());
                }
            }
            _ => {}
        }
    }

    downloads
        .run(job_id, tasks, DownloadOptions::default(), on_progress)
        .await?;

    finalize(target_dir.to_path_buf(), executables, links).await?;

    Ok(Some(
        entry
            .version
            .as_ref()
            .and_then(|version| version.name.clone())
            .unwrap_or_else(|| component.to_string()),
    ))
}

async fn finalize(
    root: PathBuf,
    executables: Vec<String>,
    links: Vec<RuntimeLink>,
) -> CommandResult<()> {
    tokio::task::spawn_blocking(move || {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            for link in &links {
                let path = join_relative(&root, &link.path);
                if path.symlink_metadata().is_ok() {
                    continue;
                }

                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| {
                        CommandError::io("Не удалось создать каталог рантайма", parent, e)
                    })?;
                }

                std::os::unix::fs::symlink(&link.target, &path)
                    .map_err(|e| CommandError::io("Не удалось создать ссылку", &path, e))?;
            }

            for executable in &executables {
                let path = join_relative(&root, executable);
                let Ok(meta) = std::fs::metadata(&path) else { continue };

                let mut perms = meta.permissions();
                perms.set_mode(perms.mode() | 0o111);

                std::fs::set_permissions(&path, perms).map_err(|e| {
                    CommandError::io("Не удалось сделать файл исполняемым", &path, e)
                })?;
            }
        }

        #[cfg(not(unix))]
        {
            let _ = (&root, &executables, &links);
        }

        Ok(())
    })
    .await
    .map_err(|e| CommandError::task_panicked("подготовка рантайма Java", e))?
}

fn join_relative(base: &Path, relative: &str) -> PathBuf {
    let mut path = base.to_path_buf();
    for part in relative.split('/').filter(|part| !part.is_empty() && *part != ".") {
        path.push(part);
    }
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(os: MojangOs, arch: &str) -> RuntimeContext {
        RuntimeContext {
            os,
            arch: arch.to_string(),
            os_version: String::new(),
        }
    }

    #[test]
    fn platform_keys_follow_jvm_architecture() {
        assert_eq!(platform_key(&ctx(MojangOs::Windows, "x86_64")), Some("windows-x64"));
        assert_eq!(platform_key(&ctx(MojangOs::Windows, "x86")), Some("windows-x86"));
        assert_eq!(platform_key(&ctx(MojangOs::Windows, "arm64")), Some("windows-arm64"));
        assert_eq!(platform_key(&ctx(MojangOs::Osx, "arm64")), Some("mac-os-arm64"));
        assert_eq!(platform_key(&ctx(MojangOs::Osx, "x86_64")), Some("mac-os"));
        assert_eq!(platform_key(&ctx(MojangOs::Linux, "x86_64")), Some("linux"));
        assert_eq!(platform_key(&ctx(MojangOs::Linux, "x86")), Some("linux-i386"));
    }

    #[test]
    fn unsupported_platforms_have_no_runtime() {
        assert_eq!(platform_key(&ctx(MojangOs::Linux, "arm64")), None);
        assert_eq!(platform_key(&ctx(MojangOs::Unknown, "x86_64")), None);
    }

    #[test]
    fn manifest_paths_are_split_into_components() {
        let path = join_relative(Path::new("/runtime"), "bin/java");
        assert!(path.ends_with(Path::new("bin").join("java")));
    }
}
