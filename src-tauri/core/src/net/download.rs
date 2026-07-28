use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use reqwest;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::{watch, Semaphore};

use crate::error::{CommandError, CommandResult};
use crate::net::http::{self, LARGE_CONCURRENCY, SMALL_CONCURRENCY};

const EMIT_INTERVAL: Duration = Duration::from_millis(100);
const FILE_PROGRESS_EPS: f64 = 0.02;
const LARGE_THRESHOLD: u64 = 4 * 1024 * 1024;
const MAX_ATTEMPTS: usize = 3;
const STALL_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadTask {
    pub url: String,
    pub destination: PathBuf,
    pub size: Option<u64>,
    pub sha1: Option<String>,
}

impl DownloadTask {
    pub fn new(url: impl Into<String>, destination: PathBuf) -> Self {
        Self {
            url: url.into(),
            destination,
            size: None,
            sha1: None,
        }
    }

    pub fn verified(
        url: impl Into<String>,
        destination: PathBuf,
        size: Option<u64>,
        sha1: Option<String>,
    ) -> Self {
        Self {
            url: url.into(),
            destination,
            size,
            sha1,
        }
    }

    fn weight(&self, use_bytes: bool) -> u64 {
        if use_bytes {
            self.size.unwrap_or(0)
        } else {
            1
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct JobError {
    pub code: String,
    pub message: String,
    pub details: Option<String>,
}

impl From<CommandError> for JobError {
    fn from(value: CommandError) -> Self {
        Self {
            code: value.code.to_string(),
            message: value.message,
            details: value.details,
        }
    }
}

impl From<JobError> for CommandError {
    fn from(value: JobError) -> Self {
        let error = CommandError::from_code(&value.code, value.message);
        match value.details {
            Some(details) => error.with_details(details),
            None => error,
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", tag = "state")]
pub enum JobStatus {
    Running,
    Finished,
    Cancelled,
    Failed { error: JobError },
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileProgress {
    pub url: String,
    pub name: String,
    pub loaded: u64,
    pub total: u64,
    pub percent: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobSnapshot {
    pub job_id: String,
    pub status: JobStatus,
    pub progress: f64,
    pub total_files: usize,
    pub done_files: usize,
    pub skipped_files: usize,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub files: Vec<FileProgress>,
}

struct Job {
    id: String,
    use_bytes: bool,
    total_weight: u64,
    done_weight: AtomicU64,
    total_files: usize,
    done_files: AtomicUsize,
    skipped_files: AtomicUsize,
    downloaded_bytes: AtomicU64,
    total_bytes: u64,
    cancel: AtomicBool,
    failed: AtomicBool,
    error: Mutex<Option<JobError>>,
    active: Mutex<HashMap<String, FileProgress>>,
    created_dirs: Mutex<HashSet<PathBuf>>,
    status: watch::Sender<JobStatus>,
    last_emit: Mutex<Instant>,
    reporter: Option<ProgressSink>,
}

impl Job {
    fn progress(&self) -> f64 {
        if self.total_weight == 0 {
            return 1.0;
        }
        (self.done_weight.load(Ordering::Relaxed) as f64 / self.total_weight as f64).clamp(0.0, 1.0)
    }

    fn snapshot(&self) -> JobSnapshot {
        JobSnapshot {
            job_id: self.id.clone(),
            status: self.status.borrow().clone(),
            progress: self.progress(),
            total_files: self.total_files,
            done_files: self.done_files.load(Ordering::Relaxed),
            skipped_files: self.skipped_files.load(Ordering::Relaxed),
            downloaded_bytes: self.downloaded_bytes.load(Ordering::Relaxed),
            total_bytes: self.total_bytes,
            files: lock(&self.active).values().cloned().collect(),
        }
    }

    fn report(&self, force: bool) {
        let Some(reporter) = &self.reporter else { return };

        {
            let mut last = lock(&self.last_emit);
            if !force && last.elapsed() < EMIT_INTERVAL {
                return;
            }
            *last = Instant::now();
        }

        reporter(&self.snapshot());
    }

    fn is_stopped(&self) -> bool {
        self.cancel.load(Ordering::Relaxed) || self.failed.load(Ordering::Relaxed)
    }

    fn fail(&self, error: CommandError) {
        if self.failed.swap(true, Ordering::SeqCst) {
            return;
        }
        *lock(&self.error) = Some(error.into());
    }

    fn add_weight(&self, delta: u64) {
        if delta > 0 {
            self.done_weight.fetch_add(delta, Ordering::Relaxed);
        }
    }

    fn set_active(&self, key: &str, progress: FileProgress) {
        lock(&self.active).insert(key.to_string(), progress);
    }

    fn clear_active(&self, key: &str) {
        lock(&self.active).remove(key);
    }

    async fn ensure_dir(&self, dir: &Path) -> CommandResult<()> {
        if lock(&self.created_dirs).contains(dir) {
            return Ok(());
        }

        crate::fs_util::ensure_dir(dir).await?;
        lock(&self.created_dirs).insert(dir.to_path_buf());

        Ok(())
    }
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

pub type ProgressSink = Box<dyn Fn(&JobSnapshot) + Send + Sync>;

#[derive(Debug, Clone, Copy, Default)]
pub struct DownloadOptions {
    pub deep_verify: bool,
}

pub struct DownloadRegistry {
    jobs: Mutex<HashMap<String, Arc<Job>>>,
    small: Arc<Semaphore>,
    large: Arc<Semaphore>,
}

impl Default for DownloadRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl DownloadRegistry {
    pub fn new() -> Self {
        Self {
            jobs: Mutex::new(HashMap::new()),
            small: Arc::new(Semaphore::new(SMALL_CONCURRENCY)),
            large: Arc::new(Semaphore::new(LARGE_CONCURRENCY)),
        }
    }

    pub fn cancel_prefix(&self, prefix: &str) {
        for (id, job) in lock(&self.jobs).iter() {
            if id.starts_with(prefix) {
                job.cancel.store(true, Ordering::SeqCst);
            }
        }
    }

    pub async fn run(
        &self,
        job_id: impl Into<String>,
        tasks: Vec<DownloadTask>,
        options: DownloadOptions,
        on_progress: Option<ProgressSink>,
    ) -> CommandResult<()> {
        let job_id = job_id.into();

        if tasks.is_empty() {
            return Ok(());
        }

        let use_bytes = tasks.iter().all(|task| task.size.unwrap_or(0) > 0);
        let total_bytes: u64 = tasks.iter().map(|task| task.size.unwrap_or(0)).sum();
        let total_weight = if use_bytes { total_bytes } else { tasks.len() as u64 };

        let (status, _) = watch::channel(JobStatus::Running);

        let job = Arc::new(Job {
            id: job_id.clone(),
            use_bytes,
            total_weight,
            done_weight: AtomicU64::new(0),
            total_files: tasks.len(),
            done_files: AtomicUsize::new(0),
            skipped_files: AtomicUsize::new(0),
            downloaded_bytes: AtomicU64::new(0),
            total_bytes,
            cancel: AtomicBool::new(false),
            failed: AtomicBool::new(false),
            error: Mutex::new(None),
            active: Mutex::new(HashMap::new()),
            created_dirs: Mutex::new(HashSet::new()),
            status,
            last_emit: Mutex::new(Instant::now() - EMIT_INTERVAL),
            reporter: on_progress,
        });

        lock(&self.jobs).insert(job_id.clone(), Arc::clone(&job));

        let result = self.execute(Arc::clone(&job), tasks, options).await;

        lock(&self.jobs).remove(&job_id);

        result
    }

    async fn execute(
        &self,
        job: Arc<Job>,
        tasks: Vec<DownloadTask>,
        options: DownloadOptions,
    ) -> CommandResult<()> {
        job.report(true);

        let client = http::client().clone();
        let mut handles = Vec::with_capacity(tasks.len());

        for task in tasks {
            let semaphore = if task.size.unwrap_or(0) >= LARGE_THRESHOLD {
                Arc::clone(&self.large)
            } else {
                Arc::clone(&self.small)
            };

            let job = Arc::clone(&job);
            let client = client.clone();

            handles.push(tokio::spawn(async move {
                if job.is_stopped() {
                    return;
                }

                let Ok(_permit) = semaphore.acquire().await else { return };

                if job.is_stopped() {
                    return;
                }

                if let Err(error) = download_one(&client, &job, &task, options).await {
                    job.fail(error);
                }
            }));
        }

        for handle in handles {
            if handle.await.is_err() && !job.cancel.load(Ordering::Relaxed) {
                job.fail(CommandError::download("Задача загрузки аварийно завершилась"));
            }
        }

        lock(&job.active).clear();

        let outcome = if job.cancel.load(Ordering::Relaxed) {
            job.status.send_replace(JobStatus::Cancelled);
            Err(CommandError::aborted("Загрузка отменена"))
        } else if let Some(error) = lock(&job.error).clone() {
            job.status.send_replace(JobStatus::Failed { error: error.clone() });
            Err(error.into())
        } else {
            job.done_weight.store(job.total_weight, Ordering::Relaxed);
            job.status.send_replace(JobStatus::Finished);
            Ok(())
        };

        job.report(true);

        outcome
    }
}

async fn download_one(
    client: &reqwest::Client,
    job: &Arc<Job>,
    task: &DownloadTask,
    options: DownloadOptions,
) -> CommandResult<()> {
    let name = file_name(&task.destination);
    let weight = task.weight(job.use_bytes);

    if is_already_valid(&task.destination, task, options.deep_verify).await {
        job.add_weight(weight);
        job.done_files.fetch_add(1, Ordering::Relaxed);
        job.skipped_files.fetch_add(1, Ordering::Relaxed);
        job.report(false);
        return Ok(());
    }

    if let Some(parent) = task.destination.parent() {
        job.ensure_dir(parent).await?;
    }

    let part_path = part_path(&task.destination);
    let mut last_error: Option<CommandError> = None;
    let mut counted: u64 = 0;

    for attempt in 1..=MAX_ATTEMPTS {
        if job.is_stopped() {
            crate::fs_util::remove_file_if_exists(&part_path).await;
            return Ok(());
        }

        if attempt > 1 {
            tokio::time::sleep(Duration::from_millis(250 * (1 << (attempt - 1)))).await;
        }

        let outcome = fetch_to_file(client, job, task, &part_path, &name, weight, &mut counted).await;
        job.clear_active(&task.url);

        match outcome {
            Ok(FetchOutcome::Completed) => {
                tokio::fs::rename(&part_path, &task.destination).await.map_err(|e| {
                    CommandError::io(
                        format!("Не удалось сохранить файл: {}", task.destination.display()),
                        &task.destination,
                        e,
                    )
                })?;

                job.done_files.fetch_add(1, Ordering::Relaxed);
                job.report(false);
                return Ok(());
            }
            Ok(FetchOutcome::Stopped) => {
                crate::fs_util::remove_file_if_exists(&part_path).await;
                return Ok(());
            }
            Err(error) => {
                crate::fs_util::remove_file_if_exists(&part_path).await;

                let retryable = matches!(error.code, "NETWORK" | "DOWNLOAD_FAILED" | "HASH_MISMATCH");
                last_error = Some(error);

                if !retryable {
                    break;
                }
            }
        }
    }

    Err(last_error
        .unwrap_or_else(|| CommandError::download(format!("Не удалось скачать {}", task.url))))
}

enum FetchOutcome {
    Completed,
    Stopped,
}

async fn fetch_to_file(
    client: &reqwest::Client,
    job: &Arc<Job>,
    task: &DownloadTask,
    part_path: &Path,
    name: &str,
    weight: u64,
    counted: &mut u64,
) -> CommandResult<FetchOutcome> {
    let mut response = client.get(&task.url).send().await.map_err(|e| {
        CommandError::network(format!("Не удалось подключиться к {}", task.url))
            .with_details(e.to_string())
    })?;

    let status = response.status();
    if !status.is_success() {
        return Err(http::http_status_error(status, &task.url));
    }

    let total = task.size.or_else(|| response.content_length()).unwrap_or(0);

    let mut file = tokio::fs::File::create(part_path).await.map_err(|e| {
        CommandError::io(
            format!("Не удалось создать файл: {}", part_path.display()),
            part_path,
            e,
        )
    })?;

    let mut hasher = Sha1::new();
    let mut received: u64 = 0;
    let mut last_percent = -1.0_f64;

    job.set_active(&task.url, FileProgress {
        url: task.url.clone(),
        name: name.to_string(),
        loaded: 0,
        total,
        percent: 0.0,
    });

    loop {
        if job.is_stopped() {
            return Ok(FetchOutcome::Stopped);
        }

        let chunk = tokio::time::timeout(STALL_TIMEOUT, response.chunk())
            .await
            .map_err(|_| {
                CommandError::network(format!("Загрузка встала: {}", task.url))
                    .with_details(format!("нет данных дольше {} с", STALL_TIMEOUT.as_secs()))
            })?
            .map_err(|e| {
                CommandError::download(format!("Обрыв загрузки: {}", task.url))
                    .with_details(e.to_string())
            })?;

        let Some(chunk) = chunk else { break };

        file.write_all(&chunk).await.map_err(|e| {
            CommandError::io(format!("Ошибка записи: {}", part_path.display()), part_path, e)
        })?;

        hasher.update(&chunk);
        received += chunk.len() as u64;
        job.downloaded_bytes.fetch_add(chunk.len() as u64, Ordering::Relaxed);

        if job.use_bytes && total > 0 {
            let target = ((received.min(total) as f64 / total as f64) * weight as f64) as u64;
            if target > *counted {
                job.add_weight(target - *counted);
                *counted = target;
            }
        }

        let percent = if total > 0 {
            (received as f64 / total as f64).min(1.0)
        } else {
            0.0
        };

        if percent - last_percent >= FILE_PROGRESS_EPS {
            last_percent = percent;
            job.set_active(&task.url, FileProgress {
                url: task.url.clone(),
                name: name.to_string(),
                loaded: received,
                total,
                percent,
            });
        }

        job.report(false);
    }

    file.flush().await.map_err(|e| {
        CommandError::io(format!("Ошибка записи: {}", part_path.display()), part_path, e)
    })?;
    drop(file);

    if let Some(expected) = &task.sha1 {
        let actual = hex(&hasher.finalize());
        if &actual != expected {
            return Err(CommandError::hash_mismatch(format!(
                "Контрольная сумма не совпала: {}",
                task.url
            ))
            .with_details(format!("Ожидалось: {expected}\nПолучено:  {actual}")));
        }
    }

    if weight > *counted {
        job.add_weight(weight - *counted);
        *counted = weight;
    }

    Ok(FetchOutcome::Completed)
}

async fn is_already_valid(destination: &Path, task: &DownloadTask, deep_verify: bool) -> bool {
    let Ok(meta) = tokio::fs::metadata(destination).await else {
        return false;
    };

    if !meta.is_file() {
        return false;
    }

    if let Some(size) = task.size {
        if meta.len() != size {
            return false;
        }
    }

    let Some(expected) = &task.sha1 else {
        return true;
    };

    if !deep_verify && task.size.is_some() {
        return true;
    }

    matches!(file_sha1(destination).await, Some(actual) if &actual == expected)
}

async fn file_sha1(path: &Path) -> Option<String> {
    let mut file = tokio::fs::File::open(path).await.ok()?;
    let mut hasher = Sha1::new();
    let mut buffer = vec![0_u8; 128 * 1024];

    loop {
        let read = file.read(&mut buffer).await.ok()?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    Some(hex(&hasher.finalize()))
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "файл".to_string())
}

fn part_path(destination: &Path) -> PathBuf {
    let name = destination
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "download".to_string());

    destination.with_file_name(format!("{name}.part"))
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";

    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(DIGITS[(byte >> 4) as usize] as char);
        out.push(DIGITS[(byte & 0x0f) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_matches_lowercase_sha1_format() {
        assert_eq!(hex(&[0x00, 0x0f, 0xa9, 0xff]), "000fa9ff");
        assert_eq!(hex(&[]), "");
    }

    #[test]
    fn part_file_sits_next_to_target() {
        assert_eq!(part_path(Path::new("/a/b/client.jar")), PathBuf::from("/a/b/client.jar.part"));
        assert_eq!(part_path(Path::new("/a/ab/abcdef")), PathBuf::from("/a/ab/abcdef.part"));
    }

    #[tokio::test]
    async fn existing_file_with_matching_size_is_skipped() {
        let dir = std::env::temp_dir().join(format!("cast-dl-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("lib.jar");
        std::fs::write(&file, b"1234").unwrap();

        let task = DownloadTask::verified("http://example/lib.jar", file.clone(), Some(4), None);
        assert!(is_already_valid(&file, &task, false).await);

        let wrong_size = DownloadTask::verified("http://example/lib.jar", file.clone(), Some(5), None);
        assert!(!is_already_valid(&file, &wrong_size, false).await);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn file_without_size_and_hash_is_trusted_as_is() {
        let dir = std::env::temp_dir().join(format!("cast-dl-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("fabric-loader.jar");
        std::fs::write(&file, b"1234").unwrap();

        let task = DownloadTask::new("http://example/fabric-loader.jar", file.clone());
        assert!(is_already_valid(&file, &task, false).await);

        let missing = DownloadTask::new("http://example/nope.jar", dir.join("nope.jar"));
        assert!(!is_already_valid(&missing.destination, &missing, false).await);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn deep_verify_rejects_corrupted_file_of_right_size() {
        let dir = std::env::temp_dir().join(format!("cast-dl-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("lib.jar");
        std::fs::write(&file, b"1234").unwrap();

        let task = DownloadTask::verified(
            "http://example/lib.jar",
            file.clone(),
            Some(4),
            Some("deadbeef".into()),
        );

        assert!(is_already_valid(&file, &task, false).await, "без deep verify доверяем размеру");
        assert!(!is_already_valid(&file, &task, true).await, "с deep verify хэш не сходится");

        std::fs::remove_dir_all(&dir).ok();
    }
}
