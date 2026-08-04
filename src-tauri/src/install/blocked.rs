use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{oneshot, Mutex};

use cast_core::error::CommandResult;
use cast_core::install::progress::ProgressReporter;
use cast_core::packs::{manual, BlockedFile};

const RESCAN_EVERY: Duration = Duration::from_secs(5);

#[derive(Default)]
pub struct BlockedRegistry {
    waiting: Mutex<HashMap<String, Waiting>>,
}

struct Waiting {
    files: Vec<BlockedFile>,
    folders: BTreeSet<PathBuf>,
    reporter: Arc<ProgressReporter>,
    resume: Option<oneshot::Sender<()>>,
}

impl BlockedRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn wait(
        &self,
        instance_id: &str,
        files: Vec<BlockedFile>,
        reporter: &Arc<ProgressReporter>,
    ) -> Vec<BlockedFile> {
        let (sender, receiver) = oneshot::channel();

        {
            let mut waiting = self.waiting.lock().await;

            waiting.insert(
                instance_id.to_string(),
                Waiting {
                    files: files.clone(),
                    folders: default_downloads_dir().into_iter().collect(),
                    reporter: Arc::clone(reporter),
                    resume: Some(sender),
                },
            );
        }

        self.rescan(instance_id).await;

        if let Some(found) = self.take_if_complete(instance_id).await {
            return found;
        }

        reporter.set_message("Ожидание файлов, которые нужно скачать вручную");
        reporter.set_awaiting_files(true);

        self.until_resumed(instance_id, receiver).await;

        reporter.set_awaiting_files(false);

        self.waiting
            .lock()
            .await
            .remove(instance_id)
            .map(|waiting| waiting.files)
            .unwrap_or(files)
    }

    async fn until_resumed(&self, instance_id: &str, mut receiver: oneshot::Receiver<()>) {
        let mut ticker = tokio::time::interval(RESCAN_EVERY);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        ticker.tick().await;

        loop {
            tokio::select! {
                _ = &mut receiver => return,
                _ = ticker.tick() => self.rescan(instance_id).await,
            }
        }
    }

    pub async fn scan(&self, instance_id: &str, dir: &Path) -> CommandResult<Vec<BlockedFile>> {
        self.remember(instance_id, dir).await;

        Ok(self.scan_dir(instance_id, dir).await)
    }

    pub async fn rescan(&self, instance_id: &str) {
        let folders = self
            .waiting
            .lock()
            .await
            .get(instance_id)
            .map(|waiting| waiting.folders.clone())
            .unwrap_or_default();

        for folder in folders {
            self.scan_dir(instance_id, &folder).await;
        }
    }

    pub async fn files(&self, instance_id: &str) -> Vec<BlockedFile> {
        self.waiting
            .lock()
            .await
            .get(instance_id)
            .map(|waiting| waiting.files.clone())
            .unwrap_or_default()
    }

    pub async fn resume(&self, instance_id: &str) {
        if let Some(waiting) = self.waiting.lock().await.get_mut(instance_id) {
            if let Some(resume) = waiting.resume.take() {
                let _ = resume.send(());
            }
        }
    }

    async fn scan_dir(&self, instance_id: &str, dir: &Path) -> Vec<BlockedFile> {
        let Some((mut files, reporter)) = self.pending(instance_id).await else {
            return Vec::new();
        };

        if manual::scan(dir, &mut files).await == 0 {
            return files;
        }

        let merged = self.merge(instance_id, &files).await;
        reporter.set_blocked(merged.clone());

        merged
    }

    async fn pending(&self, instance_id: &str) -> Option<(Vec<BlockedFile>, Arc<ProgressReporter>)> {
        let waiting = self.waiting.lock().await;
        let entry = waiting.get(instance_id)?;

        Some((entry.files.clone(), Arc::clone(&entry.reporter)))
    }

    async fn merge(&self, instance_id: &str, scanned: &[BlockedFile]) -> Vec<BlockedFile> {
        let mut waiting = self.waiting.lock().await;

        let Some(entry) = waiting.get_mut(instance_id) else {
            return scanned.to_vec();
        };

        for file in entry.files.iter_mut().filter(|file| !file.found()) {
            let Some(found) = scanned
                .iter()
                .find(|other| other.target_path == file.target_path)
            else {
                continue;
            };

            file.local_path = found.local_path.clone();
        }

        entry.files.clone()
    }

    async fn remember(&self, instance_id: &str, dir: &Path) {
        if let Some(entry) = self.waiting.lock().await.get_mut(instance_id) {
            entry.folders.insert(dir.to_path_buf());
        }
    }

    async fn take_if_complete(&self, instance_id: &str) -> Option<Vec<BlockedFile>> {
        let mut waiting = self.waiting.lock().await;

        let complete = waiting
            .get(instance_id)
            .is_some_and(|entry| entry.files.iter().all(BlockedFile::found));

        complete.then(|| waiting.remove(instance_id).map(|entry| entry.files))?
    }
}

pub fn default_downloads_dir() -> Option<std::path::PathBuf> {
    #[cfg(target_os = "windows")]
    let home = std::env::var("USERPROFILE").ok();

    #[cfg(not(target_os = "windows"))]
    let home = std::env::var("HOME").ok();

    let downloads = std::path::PathBuf::from(home?).join("Downloads");

    downloads.is_dir().then_some(downloads)
}

pub async fn already_there(path: &Path, sha1: Option<&str>) -> bool {
    if !path.is_file() {
        return false;
    }

    match sha1 {
        None => true,
        Some(expected) => {
            matches!(manual::file_sha1(path).await, Some(actual) if actual.eq_ignore_ascii_case(expected))
        }
    }
}

pub async fn place_found(minecraft: &Path, files: &[BlockedFile]) -> Vec<String> {
    let mut placed = Vec::new();

    for file in files.iter().filter(|file| file.found()) {
        match manual::place(minecraft, file).await {
            Ok(key) => placed.push(key),
            Err(error) => eprintln!("Не удалось положить скачанный файл в сборку: {error}"),
        }
    }

    placed
}
