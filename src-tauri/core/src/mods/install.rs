use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::{CommandError, CommandResult};
use crate::instance::{LoaderType, PackProvider};
use crate::net::download::{DownloadOptions, DownloadRegistry, DownloadTask};
use crate::packs::PackPage;

use super::catalog::{self, CatalogMatch, CatalogProject, CatalogVersion, Dependency};
use super::index::ModsIndex;
use super::{ModsScan, FOLDER};

const MAX_DEPTH: usize = 6;

#[derive(Debug, Clone, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ModSearch {
    pub provider: PackProvider,
    pub query: String,
    pub loader: LoaderType,
    pub game_version: String,
    pub sort: Option<String>,
    pub offset: u32,
    pub limit: u32,
}

impl Default for ModSearch {
    fn default() -> Self {
        Self {
            provider: PackProvider::Modrinth,
            query: String::new(),
            loader: LoaderType::Vanilla,
            game_version: String::new(),
            sort: None,
            offset: 0,
            limit: 20,
        }
    }
}

impl ModSearch {
    /// У vanilla-сборки фильтровать нечего: загрузчика нет.
    pub fn loader_key(&self) -> Option<&'static str> {
        match self.loader {
            LoaderType::Vanilla => None,
            other => Some(other.key()),
        }
    }

    pub fn sort_key<'a>(&'a self, supported: &[&'a str]) -> &'a str {
        self.sort
            .as_deref()
            .map(str::trim)
            .filter(|sort| supported.contains(sort))
            .or_else(|| supported.first().copied())
            .unwrap_or("relevance")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlannedMod {
    pub provider: PackProvider,
    pub project_id: String,
    pub version_id: String,
    pub version_number: String,
    pub title: String,
    pub file_name: String,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha1: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    pub icon_url: String,
    pub page_url: String,
    pub blocked: bool,
}

impl PlannedMod {
    fn matched(&self) -> CatalogMatch {
        CatalogMatch {
            provider: self.provider,
            project_id: self.project_id.clone(),
            version_id: self.version_id.clone(),
            version_number: self.version_number.clone(),
            title: self.title.clone(),
            slug: String::new(),
            icon_url: self.icon_url.clone(),
            page_url: self.page_url.clone(),
            authors: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallPlan {
    pub id: String,
    pub target: PlannedMod,
    pub required: Vec<PlannedMod>,
    pub optional: Vec<PlannedMod>,
    pub installed: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallReport {
    pub installed: Vec<String>,
    pub failed: Vec<String>,
    pub blocked: Vec<String>,
}

pub async fn search(query: &ModSearch) -> CommandResult<PackPage> {
    match query.provider {
        PackProvider::Modrinth => crate::modrinth::search_mods(query).await,
        PackProvider::CurseForge => crate::curseforge::search_mods(query).await,
    }
}

pub async fn versions(
    provider: PackProvider,
    project_id: &str,
    loader: LoaderType,
    game_version: &str,
) -> CommandResult<Vec<CatalogVersion>> {
    match provider {
        PackProvider::Modrinth => {
            crate::modrinth::mod_versions(project_id, &loader_tags(loader), game_version).await
        }
        PackProvider::CurseForge => {
            crate::curseforge::mod_versions(project_id, loader, game_version).await
        }
    }
}

pub struct PlanRequest<'a> {
    pub provider: PackProvider,
    pub project_id: &'a str,
    pub version_id: &'a str,
    pub loader: LoaderType,
    pub game_version: &'a str,
    pub installed: &'a BTreeSet<String>,
}

pub async fn plan(request: PlanRequest<'_>) -> CommandResult<InstallPlan> {
    let target = version_of(request.provider, request.project_id, request.version_id)
        .await?
        .ok_or_else(|| CommandError::version_not_found("Такой версии мода в каталоге нет"))?;

    let mut visited: BTreeSet<String> = BTreeSet::from([request.project_id.trim().to_string()]);
    let mut required: Vec<(String, CatalogVersion)> = Vec::new();
    let mut optional: Vec<(String, CatalogVersion)> = Vec::new();
    let mut installed: Vec<String> = Vec::new();

    let mut queue: Vec<(Dependency, usize)> = next_steps(&target, 0, &visited, request.installed)
        .into_iter()
        .map(|dependency| (dependency, 1))
        .collect();

    for (project_id, _) in queue.iter().map(|(dependency, _)| (dependency.project_id.clone(), ())) {
        visited.insert(project_id);
    }

    installed.extend(already_installed(&target, request.installed));

    while let Some((dependency, depth)) = queue.pop() {
        let Some(version) =
            resolve(&dependency, request.provider, request.loader, request.game_version).await
        else {
            continue;
        };

        installed.extend(already_installed(&version, request.installed));

        match dependency.required {
            true => {
                for next in next_steps(&version, depth, &visited, request.installed) {
                    visited.insert(next.project_id.clone());
                    queue.push((next, depth + 1));
                }

                required.push((dependency.project_id.clone(), version));
            }
            false => optional.push((dependency.project_id.clone(), version)),
        }
    }

    let projects = projects_info(request.provider, &ids_of(request.project_id, &required, &optional)).await;

    Ok(InstallPlan {
        id: uuid::Uuid::new_v4().simple().to_string(),
        target: planned(request.provider, request.project_id, target, &projects),
        required: required
            .into_iter()
            .map(|(project_id, version)| planned(request.provider, &project_id, version, &projects))
            .collect(),
        optional: optional
            .into_iter()
            .map(|(project_id, version)| planned(request.provider, &project_id, version, &projects))
            .collect(),
        installed: dedup(installed),
    })
}

pub(super) fn next_steps(
    version: &CatalogVersion,
    depth: usize,
    visited: &BTreeSet<String>,
    installed: &BTreeSet<String>,
) -> Vec<Dependency> {
    if depth >= MAX_DEPTH {
        return Vec::new();
    }

    version
        .dependencies
        .iter()
        .filter(|dependency| !visited.contains(&dependency.project_id))
        .filter(|dependency| !installed.contains(&dependency.project_id))
        .cloned()
        .collect()
}

fn already_installed(version: &CatalogVersion, installed: &BTreeSet<String>) -> Vec<String> {
    version
        .dependencies
        .iter()
        .filter(|dependency| installed.contains(&dependency.project_id))
        .map(|dependency| dependency.project_id.clone())
        .collect()
}

async fn resolve(
    dependency: &Dependency,
    provider: PackProvider,
    loader: LoaderType,
    game_version: &str,
) -> Option<CatalogVersion> {
    let found = match &dependency.version_id {
        Some(version_id) => version_of(provider, &dependency.project_id, version_id).await,
        None => latest(provider, &dependency.project_id, loader, game_version).await,
    };

    match found {
        Ok(version) => version,
        Err(error) => {
            eprintln!(
                "Зависимость {} не разобрана: {}",
                dependency.project_id, error.message
            );
            None
        }
    }
}

async fn version_of(
    provider: PackProvider,
    project_id: &str,
    version_id: &str,
) -> CommandResult<Option<CatalogVersion>> {
    match provider {
        PackProvider::Modrinth => crate::modrinth::mod_version(version_id).await,
        PackProvider::CurseForge => crate::curseforge::mod_version(project_id, version_id).await,
    }
}

async fn latest(
    provider: PackProvider,
    project_id: &str,
    loader: LoaderType,
    game_version: &str,
) -> CommandResult<Option<CatalogVersion>> {
    match provider {
        PackProvider::Modrinth => {
            crate::modrinth::latest_mod(project_id, &loader_tags(loader), game_version).await
        }
        PackProvider::CurseForge => {
            crate::curseforge::latest_mod(project_id, loader, game_version).await
        }
    }
}

async fn projects_info(
    provider: PackProvider,
    ids: &[String],
) -> BTreeMap<String, CatalogProject> {
    let found = match provider {
        PackProvider::Modrinth => crate::modrinth::projects_info(ids).await,
        PackProvider::CurseForge => crate::curseforge::projects_info(ids).await,
    };

    match found {
        Ok(projects) => projects,
        Err(error) => {
            eprintln!("Каталог не отдал проекты модов: {}", error.message);
            BTreeMap::new()
        }
    }
}

fn ids_of(
    target: &str,
    required: &[(String, CatalogVersion)],
    optional: &[(String, CatalogVersion)],
) -> Vec<String> {
    let mut ids = vec![target.trim().to_string()];

    ids.extend(required.iter().map(|(id, _)| id.clone()));
    ids.extend(optional.iter().map(|(id, _)| id.clone()));

    dedup(ids)
}

fn dedup(mut values: Vec<String>) -> Vec<String> {
    values.sort();
    values.dedup();
    values
}

fn planned(
    provider: PackProvider,
    project_id: &str,
    version: CatalogVersion,
    projects: &BTreeMap<String, CatalogProject>,
) -> PlannedMod {
    let project = projects.get(project_id.trim());

    PlannedMod {
        provider,
        project_id: project_id.trim().to_string(),
        version_id: version.version_id,
        version_number: version.version_number,
        title: project
            .map(|project| project.title.clone())
            .filter(|title| !title.trim().is_empty())
            .unwrap_or_else(|| version.file_name.clone()),
        file_name: version.file_name,
        url: version.url,
        sha1: version.sha1,
        size: version.size,
        icon_url: project.map(|project| project.icon_url.clone()).unwrap_or_default(),
        page_url: project.map(|project| project.page_url.clone()).unwrap_or_default(),
        blocked: version.blocked,
    }
}

fn loader_tags(loader: LoaderType) -> Vec<&'static str> {
    match loader {
        LoaderType::Vanilla => Vec::new(),
        other => vec![other.key()],
    }
}

pub fn wanted<'a>(plan: &'a InstallPlan, optional: &[String]) -> Vec<&'a PlannedMod> {
    let mut wanted = vec![&plan.target];

    wanted.extend(plan.required.iter());
    wanted.extend(
        plan.optional
            .iter()
            .filter(|planned| optional.contains(&planned.project_id)),
    );

    wanted
}

pub async fn apply(
    scan: &ModsScan,
    downloads: &DownloadRegistry,
    plan: &InstallPlan,
    optional: &[String],
    catalog_cache: &Path,
) -> CommandResult<InstallReport> {
    let mut report = InstallReport::default();

    crate::fs_util::ensure_dir(&scan.dir).await?;

    let mut index = ModsIndex::load(&scan.index_file).await;
    let mut forgotten = false;
    let mut remembered: Vec<(String, CatalogMatch)> = Vec::new();

    for (number, planned) in wanted(plan, optional).into_iter().enumerate() {
        if planned.blocked || planned.url.trim().is_empty() {
            report.blocked.push(planned.title.clone());
            continue;
        }

        let name = crate::curseforge::pack::sanitize(&planned.file_name);
        let destination = scan.dir.join(&name);

        let task = DownloadTask::verified(
            planned.url.clone(),
            destination,
            planned.size,
            planned.sha1.clone(),
        );

        let downloaded = downloads
            .run(
                format!("mods-install:{}:{number}", plan.id),
                vec![task],
                DownloadOptions::default(),
                None,
            )
            .await;

        if let Err(error) = downloaded {
            eprintln!("Не удалось поставить «{}»: {}", planned.title, error.message);
            report.failed.push(planned.title.clone());
            continue;
        }

        forgotten |= index.forget(&format!("{FOLDER}/{name}"));

        if let Some(sha1) = &planned.sha1 {
            remembered.push((sha1.to_lowercase(), planned.matched()));
        }

        report.installed.push(planned.title.clone());
    }

    if forgotten {
        let _ = index.save(&scan.index_file).await;
    }

    catalog::remember_all(catalog_cache, &remembered).await;

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn version(id: &str, dependencies: Vec<Dependency>) -> CatalogVersion {
        CatalogVersion {
            version_id: id.into(),
            version_number: "1.0.0".into(),
            file_name: "mod-1.0.0.jar".into(),
            url: "https://cdn.modrinth.com/mod-1.0.0.jar".into(),
            sha1: Some("aaa".into()),
            size: Some(10),
            date: None,
            release: "release".into(),
            blocked: false,
            dependencies,
        }
    }

    fn dependency(project_id: &str, required: bool) -> Dependency {
        Dependency {
            project_id: project_id.into(),
            version_id: None,
            required,
        }
    }

    fn planned_mod(project_id: &str, blocked: bool) -> PlannedMod {
        PlannedMod {
            provider: PackProvider::Modrinth,
            project_id: project_id.into(),
            version_id: "v1".into(),
            version_number: "1.0.0".into(),
            title: project_id.into(),
            file_name: format!("{project_id}.jar"),
            url: "https://cdn.modrinth.com/x.jar".into(),
            sha1: Some("aaa".into()),
            size: Some(10),
            icon_url: String::new(),
            page_url: String::new(),
            blocked,
        }
    }

    fn plan_of(required: Vec<&str>, optional: Vec<&str>) -> InstallPlan {
        InstallPlan {
            id: "plan".into(),
            target: planned_mod("target", false),
            required: required.into_iter().map(|id| planned_mod(id, false)).collect(),
            optional: optional.into_iter().map(|id| planned_mod(id, false)).collect(),
            installed: Vec::new(),
        }
    }

    #[test]
    fn dependencies_of_a_version_become_the_next_steps() {
        let version = version("v1", vec![dependency("fabric-api", true), dependency("sodium", false)]);

        let steps = next_steps(&version, 0, &BTreeSet::new(), &BTreeSet::new());

        assert_eq!(steps.len(), 2);
        assert!(steps.iter().any(|step| step.project_id == "fabric-api" && step.required));
    }

    #[test]
    fn what_is_already_installed_is_not_pulled_again() {
        let version = version("v1", vec![dependency("fabric-api", true)]);
        let installed = BTreeSet::from(["fabric-api".to_string()]);

        assert!(next_steps(&version, 0, &BTreeSet::new(), &installed).is_empty());
        assert_eq!(already_installed(&version, &installed), vec!["fabric-api"]);
    }

    #[test]
    fn a_project_seen_once_is_not_visited_twice() {
        let version = version("v1", vec![dependency("fabric-api", true)]);
        let visited = BTreeSet::from(["fabric-api".to_string()]);

        assert!(next_steps(&version, 0, &visited, &BTreeSet::new()).is_empty());
    }

    #[test]
    fn the_walk_stops_at_a_sane_depth() {
        let version = version("v1", vec![dependency("deep", true)]);

        assert!(!next_steps(&version, MAX_DEPTH - 1, &BTreeSet::new(), &BTreeSet::new()).is_empty());
        assert!(next_steps(&version, MAX_DEPTH, &BTreeSet::new(), &BTreeSet::new()).is_empty());
    }

    #[test]
    fn only_the_ticked_optional_mods_are_installed() {
        let plan = plan_of(vec!["fabric-api"], vec!["sodium", "iris"]);

        let names: Vec<&str> = wanted(&plan, &["iris".to_string()])
            .iter()
            .map(|planned| planned.project_id.as_str())
            .collect();

        assert_eq!(names, vec!["target", "fabric-api", "iris"]);
    }

    #[test]
    fn without_ticks_only_the_mod_and_its_required_deps_go() {
        let plan = plan_of(vec!["fabric-api"], vec!["sodium"]);

        let names: Vec<&str> = wanted(&plan, &[])
            .iter()
            .map(|planned| planned.project_id.as_str())
            .collect();

        assert_eq!(names, vec!["target", "fabric-api"]);
    }

    #[test]
    fn the_search_falls_back_to_a_sort_the_provider_knows() {
        let mut query = ModSearch {
            sort: Some("follows".into()),
            ..ModSearch::default()
        };

        assert_eq!(query.sort_key(&["relevance", "downloads", "follows"]), "follows");
        assert_eq!(query.sort_key(&["relevance", "downloads"]), "relevance");

        query.sort = None;
        assert_eq!(query.sort_key(&["relevance", "downloads"]), "relevance");
    }

    #[test]
    fn a_vanilla_instance_has_no_loader_to_filter_by() {
        let fabric = ModSearch {
            loader: LoaderType::Fabric,
            ..ModSearch::default()
        };

        assert_eq!(fabric.loader_key(), Some("fabric"));
        assert_eq!(ModSearch::default().loader_key(), None);
        assert_eq!(loader_tags(LoaderType::NeoForge), vec!["neoforge"]);
        assert!(loader_tags(LoaderType::Vanilla).is_empty());
    }

    #[test]
    fn a_planned_mod_knows_what_to_write_into_the_catalog_cache() {
        let matched = planned_mod("sodium", false).matched();

        assert_eq!(matched.project_id, "sodium");
        assert_eq!(matched.version_id, "v1");
        assert_eq!(matched.provider, PackProvider::Modrinth);
    }

    #[tokio::test]
    async fn a_blocked_file_is_reported_instead_of_downloaded() {
        let root = std::env::temp_dir().join(format!("cast-install-{}", uuid::Uuid::new_v4()));
        let scan = ModsScan {
            dir: root.join("mods"),
            index_file: root.join("mods-index.json"),
            icons: root.join("icons"),
            loader: None,
            managed: Default::default(),
        };

        let mut plan = plan_of(Vec::new(), Vec::new());
        plan.target = planned_mod("blocked-mod", true);

        let report = apply(&scan, &DownloadRegistry::new(), &plan, &[], &root.join("catalog.json"))
            .await
            .unwrap();

        assert_eq!(report.blocked, vec!["blocked-mod"]);
        assert!(report.installed.is_empty());

        std::fs::remove_dir_all(&root).ok();
    }
}
