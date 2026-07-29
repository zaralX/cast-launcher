use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use tokio::sync::{oneshot, Mutex};

use cast_core::error::CommandResult;
use cast_core::install::progress::ProgressReporter;
use cast_core::packs::{manual, BlockedFile};

#[derive(Default)]
pub struct BlockedRegistry {
    waiting: Mutex<HashMap<String, Waiting>>,
}

struct Waiting {
    files: Vec<BlockedFile>,
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
                    reporter: Arc::clone(reporter),
                    resume: Some(sender),
                },
            );
        }

        self.rescan_default(instance_id).await;

        if let Some(found) = self.take_if_complete(instance_id).await {
            return found;
        }

        reporter.set_message("Ожидание файлов, которые нужно скачать вручную");
        reporter.set_awaiting_files(true);

        let _ = receiver.await;

        reporter.set_awaiting_files(false);

        self.waiting
            .lock()
            .await
            .remove(instance_id)
            .map(|waiting| waiting.files)
            .unwrap_or(files)
    }

    pub async fn scan(&self, instance_id: &str, dir: &Path) -> CommandResult<Vec<BlockedFile>> {
        let mut waiting = self.waiting.lock().await;

        let Some(entry) = waiting.get_mut(instance_id) else {
            return Ok(Vec::new());
        };

        manual::scan(dir, &mut entry.files).await;

        let files = entry.files.clone();
        entry.reporter.set_blocked(files.clone());

        Ok(files)
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

    async fn take_if_complete(&self, instance_id: &str) -> Option<Vec<BlockedFile>> {
        let mut waiting = self.waiting.lock().await;

        let complete = waiting
            .get(instance_id)
            .is_some_and(|entry| entry.files.iter().all(BlockedFile::found));

        complete.then(|| waiting.remove(instance_id).map(|entry| entry.files))?
    }

    async fn rescan_default(&self, instance_id: &str) {
        let Some(dir) = default_downloads_dir() else { return };

        let _ = self.scan(instance_id, &dir).await;
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
