use std::collections::BTreeMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};
use zip::ZipWriter;

use crate::castpack::export::{is_catalog_candidate, verify, ExportFile, ExportRequest};
use crate::castpack::file::{entry_options, zip_error, EXTENSION as CAST_EXTENSION};
use crate::curseforge::pack::{
    self as curseforge_pack, ManifestFile, Minecraft, ModLoader, MANIFEST_TYPE,
};
use crate::error::{CommandError, CommandResult};
use crate::instance::{Instance, LoaderType};
use crate::modrinth::pack::{self as modrinth_pack, FileEnv, PackIndex};
use crate::mods::catalog::CatalogMatch;
use crate::mods::hash::FileHashes;
use crate::mods::{self, DISABLED_SUFFIX};

use super::PackFile;

pub const OVERRIDES: &str = "overrides";

const BATCH: usize = 300;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    #[default]
    Cast,
    Mrpack,
    CurseForge,
}

impl ExportFormat {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Cast => CAST_EXTENSION,
            Self::Mrpack => "mrpack",
            Self::CurseForge => "zip",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Cast => "cast",
            Self::Mrpack => "mrpack",
            Self::CurseForge => "curseforge",
        }
    }
}

#[derive(Debug)]
pub struct Plan {
    pub manifest_entry: &'static str,
    pub manifest: Vec<u8>,
    pub links: usize,
    pub overrides: Vec<ExportFile>,
}

impl Plan {
    pub fn size(&self) -> u64 {
        self.overrides.iter().map(|file| file.size).sum()
    }

    pub fn embedded_code(&self) -> usize {
        let prefix = format!("{}/", mods::FOLDER);

        self.overrides
            .iter()
            .filter(|file| file.key.starts_with(&prefix) && is_catalog_candidate(&file.key))
            .count()
    }
}

pub async fn modrinth_links(
    hashes: &BTreeMap<String, FileHashes>,
) -> CommandResult<BTreeMap<String, PackFile>> {
    let mut sha1s: Vec<String> = hashes
        .values()
        .map(|found| found.sha1.to_ascii_lowercase())
        .collect();
    sha1s.sort_unstable();
    sha1s.dedup();

    let mut found = BTreeMap::new();

    for chunk in sha1s.chunks(BATCH) {
        found.extend(crate::modrinth::exact_files_by_sha1(chunk).await?);
    }

    Ok(hashes
        .iter()
        .filter_map(|(key, hashes)| {
            let file = found.get(&hashes.sha1.to_ascii_lowercase())?;

            (!file.url.is_empty() && file.hashes.sha512.is_some())
                .then(|| (key.clone(), file.clone()))
        })
        .collect())
}

pub async fn curseforge_links(
    hashes: &BTreeMap<String, FileHashes>,
    minecraft_dir: &Path,
) -> CommandResult<BTreeMap<String, CatalogMatch>> {
    let mut by_fingerprint: BTreeMap<u32, Vec<&String>> = BTreeMap::new();

    for (key, found) in hashes {
        if let Some(fingerprint) = found.fingerprint {
            by_fingerprint.entry(fingerprint).or_default().push(key);
        }
    }

    let fingerprints: Vec<u32> = by_fingerprint.keys().copied().collect();
    let mut matched = BTreeMap::new();

    for chunk in fingerprints.chunks(BATCH) {
        for (fingerprint, found) in crate::curseforge::identify(chunk).await? {
            for key in by_fingerprint.get(&fingerprint).into_iter().flatten() {
                matched.insert((*key).clone(), found.clone());
            }
        }
    }

    Ok(verify(matched, hashes, minecraft_dir).await?.0)
}

pub fn mrpack(
    instance: &Instance,
    request: &ExportRequest,
    files: Vec<ExportFile>,
    links: &BTreeMap<String, PackFile>,
) -> CommandResult<Plan> {
    let mut index_files = Vec::new();
    let mut overrides = Vec::new();

    for file in files {
        let Some(link) = links.get(&file.key) else {
            overrides.push(file);
            continue;
        };

        let (path, env) = match file.key.strip_suffix(DISABLED_SUFFIX) {
            Some(path) => (path.to_string(), "optional"),
            None => (file.key.clone(), "required"),
        };

        let hashes = [("sha1", &link.hashes.sha1), ("sha512", &link.hashes.sha512)]
            .into_iter()
            .filter_map(|(name, hash)| Some((name.to_string(), hash.clone()?)))
            .collect();

        index_files.push(modrinth_pack::PackFile {
            path,
            hashes,
            env: Some(FileEnv {
                client: Some(env.to_string()),
                server: Some(env.to_string()),
            }),
            downloads: vec![link.url.clone()],
            file_size: Some(link.size.unwrap_or(file.size)),
        });
    }

    index_files.sort_by(|a, b| a.path.cmp(&b.path));

    let mut dependencies =
        BTreeMap::from([("minecraft".to_string(), instance.minecraft_version.clone())]);

    if let Some((loader, version)) = loader(instance)? {
        let name = match loader {
            LoaderType::Fabric => "fabric-loader",
            LoaderType::Forge => "forge",
            LoaderType::NeoForge => "neoforge",
            LoaderType::Vanilla => unreachable!("loader() gives no version for vanilla"),
        };

        dependencies.insert(name.to_string(), version);
    }

    let index = PackIndex {
        format_version: 1,
        game: "minecraft".to_string(),
        version_id: request.version.clone(),
        name: request.name.clone(),
        summary: Some(request.description.clone()).filter(|summary| !summary.is_empty()),
        files: index_files,
        dependencies,
    };

    Ok(Plan {
        manifest_entry: modrinth_pack::INDEX_ENTRY,
        manifest: to_json(&index)?,
        links: index.files.len(),
        overrides,
    })
}

pub fn curseforge(
    instance: &Instance,
    request: &ExportRequest,
    files: Vec<ExportFile>,
    links: &BTreeMap<String, CatalogMatch>,
) -> CommandResult<Plan> {
    let mut manifest_files = Vec::new();
    let mut overrides = Vec::new();

    for file in files {
        let entry = links.get(&file.key).and_then(|found| {
            Some(ManifestFile {
                project_id: found.project_id.parse().ok()?,
                file_id: found.version_id.parse().ok()?,
                required: !file.key.ends_with(DISABLED_SUFFIX),
            })
        });

        match entry {
            Some(entry) => manifest_files.push(entry),
            None => overrides.push(file),
        }
    }

    manifest_files.sort_by_key(|file| (file.project_id, file.file_id));

    let mod_loaders = loader(instance)?
        .map(|(loader, version)| {
            let id = match loader {
                LoaderType::Fabric => format!("fabric-{version}"),
                LoaderType::Forge => format!("forge-{version}"),
                LoaderType::NeoForge if instance.minecraft_version == "1.20.1" => {
                    format!("neoforge-1.20.1-{version}")
                }
                LoaderType::NeoForge => format!("neoforge-{version}"),
                LoaderType::Vanilla => unreachable!("loader() gives no version for vanilla"),
            };

            ModLoader { id, primary: true }
        })
        .into_iter()
        .collect();

    let manifest = curseforge_pack::Manifest {
        manifest_type: MANIFEST_TYPE.to_string(),
        manifest_version: 1,
        name: request.name.clone(),
        version: request.version.clone(),
        author: request.author.clone(),
        minecraft: Minecraft {
            version: instance.minecraft_version.clone(),
            mod_loaders,
            recommended_ram: request.recommended_ram,
        },
        files: manifest_files,
        overrides: OVERRIDES.to_string(),
    };

    Ok(Plan {
        manifest_entry: curseforge_pack::MANIFEST_ENTRY,
        manifest: to_json(&manifest)?,
        links: manifest.files.len(),
        overrides,
    })
}

fn loader(instance: &Instance) -> CommandResult<Option<(LoaderType, String)>> {
    if instance.loader == LoaderType::Vanilla {
        return Ok(None);
    }

    let version = instance.require_loader_version()?;
    let prefix = format!("{}-", instance.minecraft_version);

    Ok(Some((
        instance.loader,
        version.strip_prefix(&prefix).unwrap_or(version).to_string(),
    )))
}

fn to_json(value: &impl Serialize) -> CommandResult<Vec<u8>> {
    serde_json::to_vec_pretty(value).map_err(|e| {
        CommandError::unknown("error.reason.internal.serialize").with_details(e.to_string())
    })
}

pub fn write(
    target: &Path,
    plan: &Plan,
    progress: &dyn Fn(u64),
    cancelled: &dyn Fn() -> bool,
) -> CommandResult<()> {
    let out = File::create(target)
        .map_err(|e| CommandError::io("error.reason.fs.create_file", target, e))?;
    let mut zip = ZipWriter::new(out);

    zip.start_file(
        plan.manifest_entry,
        entry_options(plan.manifest_entry, plan.manifest.len() as u64),
    )
    .map_err(|e| zip_error(target, e))?;
    zip.write_all(&plan.manifest)
        .map_err(|e| CommandError::io("error.reason.fs.write_file", target, e))?;

    let mut written = 0u64;
    let mut chunk = vec![0u8; 256 * 1024];

    for file in &plan.overrides {
        if cancelled() {
            return Err(CommandError::aborted("error.reason.cast.export_cancelled"));
        }

        let name = format!("{OVERRIDES}/{}", file.key);
        let mut source = File::open(&file.path)
            .map_err(|e| CommandError::io("error.reason.fs.read_file", &file.path, e))?;

        zip.start_file(&name, entry_options(&name, file.size))
            .map_err(|e| zip_error(target, e))?;

        loop {
            let read = source
                .read(&mut chunk)
                .map_err(|e| CommandError::io("error.reason.fs.read_file", &file.path, e))?;

            if read == 0 {
                break;
            }

            zip.write_all(&chunk[..read])
                .map_err(|e| CommandError::io("error.reason.fs.write_file", target, e))?;

            written += read as u64;
            progress(written);
        }
    }

    let mut out = zip.finish().map_err(|e| zip_error(target, e))?;
    out.flush()
        .map_err(|e| CommandError::io("error.reason.fs.write_file", target, e))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::castpack::manifest::FileMode;
    use crate::instance::PackProvider;
    use crate::packs::FileHashes as LinkHashes;

    fn instance(loader: &str, minecraft: &str, version: Option<&str>) -> Instance {
        serde_json::from_value(serde_json::json!({
            "id": "abc",
            "name": "Сборка",
            "minecraftVersion": minecraft,
            "type": loader,
            "loaderVersion": version,
        }))
        .unwrap()
    }

    fn request() -> ExportRequest {
        ExportRequest {
            name: "Мой пак".into(),
            version: "1.0.0".into(),
            author: "zaralX".into(),
            description: "Описание".into(),
            recommended_ram: Some(6144),
            include: vec!["mods".into()],
            ..Default::default()
        }
    }

    fn file(key: &str) -> ExportFile {
        ExportFile {
            key: key.into(),
            path: PathBuf::from(key),
            size: 7,
            mode: FileMode::Always,
        }
    }

    fn modrinth_file(url: &str) -> PackFile {
        PackFile {
            url: url.into(),
            filename: "sodium.jar".into(),
            size: Some(100),
            hashes: LinkHashes {
                sha1: Some("aaa".into()),
                sha512: Some("bbb".into()),
            },
        }
    }

    fn curseforge_match(project: &str, file: &str) -> CatalogMatch {
        CatalogMatch {
            provider: PackProvider::CurseForge,
            project_id: project.into(),
            version_id: file.into(),
            version_number: String::new(),
            title: String::new(),
            slug: String::new(),
            icon_url: String::new(),
            page_url: String::new(),
            authors: Vec::new(),
        }
    }

    #[test]
    fn an_mrpack_links_modrinth_files_and_carries_the_rest() {
        let links = BTreeMap::from([
            (
                "mods/sodium.jar".to_string(),
                modrinth_file("https://cdn.modrinth.com/sodium.jar"),
            ),
            (
                "mods/zoom.jar.disabled".to_string(),
                modrinth_file("https://cdn.modrinth.com/zoom.jar"),
            ),
        ]);

        let plan = mrpack(
            &instance("fabric", "1.20.1", Some("0.16.5")),
            &request(),
            vec![
                file("mods/sodium.jar"),
                file("mods/zoom.jar.disabled"),
                file("mods/curseforge-only.jar"),
                file("config/sodium.json"),
            ],
            &links,
        )
        .unwrap();

        assert_eq!(plan.manifest_entry, "modrinth.index.json");
        assert_eq!(plan.links, 2);
        assert_eq!(plan.overrides.len(), 2);
        assert_eq!(
            plan.embedded_code(),
            1,
            "only the jar counts as a mod inside"
        );
        assert_eq!(plan.size(), 14);

        let index = PackIndex::parse(&plan.manifest).unwrap();
        assert_eq!(index.name, "Мой пак");
        assert_eq!(index.version_id, "1.0.0");
        assert_eq!(index.summary.as_deref(), Some("Описание"));
        assert_eq!(
            index.loader().unwrap(),
            (LoaderType::Fabric, Some("0.16.5".to_string()))
        );
        assert_eq!(
            index.client_paths().unwrap(),
            vec!["mods/sodium.jar", "mods/zoom.jar.disabled"],
            "a disabled mod comes back disabled"
        );
        assert_eq!(index.files[0].sha1().as_deref(), Some("aaa"));
        assert_eq!(index.files[0].hashes["sha512"], "bbb");
        assert_eq!(index.files[0].file_size, Some(100));
    }

    #[test]
    fn a_curseforge_pack_lists_project_and_file_ids() {
        let links = BTreeMap::from([
            (
                "mods/jei.jar".to_string(),
                curseforge_match("238222", "4712866"),
            ),
            (
                "mods/off.jar.disabled".to_string(),
                curseforge_match("100", "200"),
            ),
            ("mods/broken.jar".to_string(), curseforge_match("x", "1")),
        ]);

        let plan = curseforge(
            &instance("forge", "1.20.1", Some("1.20.1-47.2.0")),
            &request(),
            vec![
                file("mods/jei.jar"),
                file("mods/off.jar.disabled"),
                file("mods/broken.jar"),
                file("options.txt"),
            ],
            &links,
        )
        .unwrap();

        assert_eq!(plan.manifest_entry, "manifest.json");
        assert_eq!(plan.links, 2);
        assert_eq!(
            plan.overrides
                .iter()
                .map(|file| file.key.as_str())
                .collect::<Vec<_>>(),
            vec!["mods/broken.jar", "options.txt"]
        );

        let manifest = curseforge_pack::Manifest::parse(&plan.manifest).unwrap();
        assert_eq!(manifest.author, "zaralX");
        assert_eq!(manifest.overrides, "overrides");
        assert_eq!(manifest.minecraft.recommended_ram, Some(6144));
        assert_eq!(manifest.minecraft.mod_loaders[0].id, "forge-47.2.0");
        assert_eq!(
            manifest.loader().unwrap(),
            (LoaderType::Forge, Some("1.20.1-47.2.0".to_string()))
        );
        assert_eq!(manifest.files[0].project_id, 100);
        assert!(!manifest.files[0].required);
        assert_eq!(manifest.files[1].file_id, 4712866);
        assert!(manifest.files[1].required);
    }

    #[test]
    fn loader_versions_survive_a_round_trip_through_both_formats() {
        for (loader, minecraft, version) in [
            ("neoforge", "1.20.1", "1.20.1-47.1.106"),
            ("neoforge", "1.21.1", "21.1.77"),
            ("forge", "1.12.2", "1.12.2-14.23.5.2860"),
            ("fabric", "1.21.1", "0.16.5"),
        ] {
            let instance = instance(loader, minecraft, Some(version));
            let expected = (instance.loader, Some(version.to_string()));

            let plan = mrpack(&instance, &request(), Vec::new(), &BTreeMap::new()).unwrap();
            assert_eq!(
                PackIndex::parse(&plan.manifest).unwrap().loader().unwrap(),
                expected,
                "mrpack, {loader} {version}"
            );

            let plan = curseforge(&instance, &request(), Vec::new(), &BTreeMap::new()).unwrap();
            assert_eq!(
                curseforge_pack::Manifest::parse(&plan.manifest)
                    .unwrap()
                    .loader()
                    .unwrap(),
                expected,
                "curseforge, {loader} {version}"
            );
        }
    }

    #[test]
    fn a_vanilla_pack_names_only_minecraft_and_a_loader_needs_its_version() {
        let plan = mrpack(
            &instance("vanilla", "1.21.1", None),
            &request(),
            Vec::new(),
            &BTreeMap::new(),
        )
        .unwrap();
        let index = PackIndex::parse(&plan.manifest).unwrap();
        assert_eq!(index.dependencies.len(), 1);
        assert_eq!(index.loader().unwrap(), (LoaderType::Vanilla, None));

        assert!(mrpack(
            &instance("fabric", "1.21.1", None),
            &request(),
            Vec::new(),
            &BTreeMap::new(),
        )
        .is_err());
    }

    #[test]
    fn the_archive_holds_the_manifest_and_the_overrides() {
        let dir = std::env::temp_dir().join(format!("cast-pack-export-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();

        let source = dir.join("sodium.json");
        std::fs::write(&source, b"{}").unwrap();

        let plan = Plan {
            manifest_entry: "manifest.json",
            manifest: b"{\"a\":1}".to_vec(),
            links: 0,
            overrides: vec![ExportFile {
                key: "config/sodium.json".into(),
                path: source,
                size: 2,
                mode: FileMode::Always,
            }],
        };

        let target = dir.join("pack.zip");
        write(&target, &plan, &|_| {}, &|| false).unwrap();

        let mut archive = zip::ZipArchive::new(File::open(&target).unwrap()).unwrap();
        let mut names: Vec<String> = (0..archive.len())
            .map(|index| archive.by_index(index).unwrap().name().unwrap().to_string())
            .collect();
        names.sort();
        assert_eq!(names, vec!["manifest.json", "overrides/config/sodium.json"]);

        let mut text = String::new();
        archive
            .by_name("overrides/config/sodium.json")
            .unwrap()
            .read_to_string(&mut text)
            .unwrap();
        assert_eq!(text, "{}");

        let cancelled = write(&dir.join("cancelled.zip"), &plan, &|_| {}, &|| true);
        assert!(cancelled.is_err());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn formats_have_their_extensions() {
        assert_eq!(ExportFormat::Cast.extension(), "cast");
        assert_eq!(ExportFormat::Mrpack.extension(), "mrpack");
        assert_eq!(ExportFormat::CurseForge.extension(), "zip");
        assert_eq!(
            serde_json::from_str::<ExportFormat>("\"curseforge\"").unwrap(),
            ExportFormat::CurseForge
        );
    }
}
