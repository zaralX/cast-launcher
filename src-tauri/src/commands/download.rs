use crate::error::CommandError;
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_http::reqwest;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::{watch, Semaphore};

const PROGRESS_EVENT: &str = "download:progress";
const EMIT_INTERVAL: Duration = Duration::from_millis(100);
const FILE_PROGRESS_EPS: f64 = 0.02;

const DEFAULT_SMALL_CONCURRENCY: usize = 24;
const DEFAULT_LARGE_CONCURRENCY: usize = 4;
const DEFAULT_LARGE_THRESHOLD: u64 = 4 * 1024 * 1024;
const MAX_ATTEMPTS: usize = 3;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadTask {
    pub url: String,
    pub destination: String,
    pub size: Option<u64>,
    pub verification_type: Option<String>,
    pub hash: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadOptions {
    pub large_threshold: Option<u64>,
    pub deep_verify: Option<bool>,
}

struct ResolvedOptions {
    large_threshold: u64,
    deep_verify: bool,
}

impl From<Option<DownloadOptions>> for ResolvedOptions {
    fn from(value: Option<DownloadOptions>) -> Self {
        let value = value.unwrap_or_default();
        Self {
            large_threshold: value.large_threshold.unwrap_or(DEFAULT_LARGE_THRESHOLD),
            deep_verify: value.deep_verify.unwrap_or(false),
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

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", tag = "state")]
pub enum JobStatus {
    Running,
    Finished,
    Cancelled,
    Failed { error: JobError },
}

impl JobStatus {
    fn is_terminal(&self) -> bool {
        !matches!(self, JobStatus::Running)
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileProgress {
    pub url: String,
    pub name: String,
    pub destination: String,
    pub loaded: u64,
    pub total: u64,
    pub percent: f64,
    pub done: bool,
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
}

impl Job {
    fn progress(&self) -> f64 {
        if self.total_weight == 0 {
            return 1.0;
        }
        (self.done_weight.load(Ordering::Relaxed) as f64 / self.total_weight as f64).clamp(0.0, 1.0)
    }

    fn snapshot(&self) -> JobSnapshot {
        let files = match self.active.lock() {
            Ok(active) => active.values().cloned().collect(),
            Err(poisoned) => poisoned.into_inner().values().cloned().collect(),
        };

        JobSnapshot {
            job_id: self.id.clone(),
            status: self.status.borrow().clone(),
            progress: self.progress(),
            total_files: self.total_files,
            done_files: self.done_files.load(Ordering::Relaxed),
            skipped_files: self.skipped_files.load(Ordering::Relaxed),
            downloaded_bytes: self.downloaded_bytes.load(Ordering::Relaxed),
            total_bytes: self.total_bytes,
            files,
        }
    }

    fn emit(&self, app: &AppHandle, force: bool) {
        {
            let mut last = match self.last_emit.lock() {
                Ok(last) => last,
                Err(poisoned) => poisoned.into_inner(),
            };
            if !force && last.elapsed() < EMIT_INTERVAL {
                return;
            }
            *last = Instant::now();
        }

        let _ = app.emit(PROGRESS_EVENT, self.snapshot());
    }

    fn is_stopped(&self) -> bool {
        self.cancel.load(Ordering::Relaxed) || self.failed.load(Ordering::Relaxed)
    }

    fn fail(&self, error: CommandError) {
        if self.failed.swap(true, Ordering::SeqCst) {
            return;
        }
        if let Ok(mut slot) = self.error.lock() {
            *slot = Some(error.into());
        }
    }

    fn add_weight(&self, delta: u64) {
        if delta > 0 {
            self.done_weight.fetch_add(delta, Ordering::Relaxed);
        }
    }

    fn set_active(&self, key: &str, progress: FileProgress) {
        if let Ok(mut active) = self.active.lock() {
            active.insert(key.to_string(), progress);
        }
    }

    fn clear_active(&self, key: &str) {
        if let Ok(mut active) = self.active.lock() {
            active.remove(key);
        }
    }

    async fn ensure_dir(&self, dir: &Path) -> Result<(), CommandError> {
        {
            let known = match self.created_dirs.lock() {
                Ok(known) => known,
                Err(poisoned) => poisoned.into_inner(),
            };
            if known.contains(dir) {
                return Ok(());
            }
        }

        tokio::fs::create_dir_all(dir).await.map_err(|e| {
            CommandError::fs(format!("Не удалось создать каталог: {}", dir.display()))
                .with_details(e.to_string())
        })?;

        if let Ok(mut known) = self.created_dirs.lock() {
            known.insert(dir.to_path_buf());
        }

        Ok(())
    }
}

pub struct DownloadRegistry {
    client: Mutex<Option<reqwest::Client>>,
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
            client: Mutex::new(None),
            jobs: Mutex::new(HashMap::new()),
            small: Arc::new(Semaphore::new(DEFAULT_SMALL_CONCURRENCY)),
            large: Arc::new(Semaphore::new(DEFAULT_LARGE_CONCURRENCY)),
        }
    }

    fn client(&self) -> reqwest::Client {
        let mut slot = match self.client.lock() {
            Ok(slot) => slot,
            Err(poisoned) => poisoned.into_inner(),
        };

        if let Some(client) = slot.as_ref() {
            return client.clone();
        }

        let client = reqwest::Client::builder()
            .pool_max_idle_per_host(DEFAULT_SMALL_CONCURRENCY * 2)
            .pool_idle_timeout(Duration::from_secs(90))
            .connect_timeout(Duration::from_secs(15))
            .user_agent("cast-launcher")
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        *slot = Some(client.clone());
        client
    }

    fn get(&self, job_id: &str) -> Option<Arc<Job>> {
        let jobs = match self.jobs.lock() {
            Ok(jobs) => jobs,
            Err(poisoned) => poisoned.into_inner(),
        };
        jobs.get(job_id).cloned()
    }

    fn forget(&self, job_id: &str) {
        let mut jobs = match self.jobs.lock() {
            Ok(jobs) => jobs,
            Err(poisoned) => poisoned.into_inner(),
        };
        jobs.remove(job_id);
    }
}

#[tauri::command]
pub async fn start_download(
    app: AppHandle,
    registry: State<'_, DownloadRegistry>,
    job_id: String,
    tasks: Vec<DownloadTask>,
    options: Option<DownloadOptions>,
) -> Result<JobSnapshot, CommandError> {
    if let Some(existing) = registry.get(&job_id) {
        if !existing.status.borrow().is_terminal() {
            return Ok(existing.snapshot());
        }
    }

    let options: ResolvedOptions = options.into();

    let use_bytes = !tasks.is_empty() && tasks.iter().all(|t| t.size.unwrap_or(0) > 0);
    let total_bytes = tasks.iter().map(|t| t.size.unwrap_or(0)).sum();
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
    });

    {
        let mut jobs = match registry.jobs.lock() {
            Ok(jobs) => jobs,
            Err(poisoned) => poisoned.into_inner(),
        };
        jobs.insert(job_id.clone(), Arc::clone(&job));
    }

    let snapshot = job.snapshot();
    let client = registry.client();
    let limits = Limits {
        small: Arc::clone(&registry.small),
        large: Arc::clone(&registry.large),
    };

    tauri::async_runtime::spawn(run_job(app, client, limits, job, tasks, options));

    Ok(snapshot)
}

#[tauri::command]
pub async fn await_download(
    registry: State<'_, DownloadRegistry>,
    job_id: String,
) -> Result<(), CommandError> {
    let Some(job) = registry.get(&job_id) else {
        return Ok(());
    };

    let mut rx = job.status.subscribe();

    loop {
        let status = rx.borrow_and_update().clone();

        match status {
            JobStatus::Running => {}
            JobStatus::Finished => {
                registry.forget(&job_id);
                return Ok(());
            }
            JobStatus::Cancelled => {
                registry.forget(&job_id);
                return Err(CommandError::aborted("Загрузка отменена"));
            }
            JobStatus::Failed { error } => {
                registry.forget(&job_id);

                let mut command_error = CommandError::from_code(&error.code, error.message);
                if let Some(details) = error.details {
                    command_error = command_error.with_details(details);
                }
                return Err(command_error);
            }
        }

        if rx.changed().await.is_err() {
            return Ok(());
        }
    }
}

#[tauri::command]
pub fn cancel_download(registry: State<'_, DownloadRegistry>, job_id: String) {
    if let Some(job) = registry.get(&job_id) {
        job.cancel.store(true, Ordering::SeqCst);
    }
}

#[tauri::command]
pub fn list_downloads(registry: State<'_, DownloadRegistry>) -> Vec<JobSnapshot> {
    let jobs = match registry.jobs.lock() {
        Ok(jobs) => jobs,
        Err(poisoned) => poisoned.into_inner(),
    };
    jobs.values().map(|job| job.snapshot()).collect()
}

struct Limits {
    small: Arc<Semaphore>,
    large: Arc<Semaphore>,
}

async fn run_job(
    app: AppHandle,
    client: reqwest::Client,
    limits: Limits,
    job: Arc<Job>,
    tasks: Vec<DownloadTask>,
    options: ResolvedOptions,
) {
    let Limits { small, large } = limits;
    let deep_verify = options.deep_verify;

    job.emit(&app, true);

    let mut handles = Vec::with_capacity(tasks.len());

    for task in tasks {
        let semaphore = if task.size.unwrap_or(0) >= options.large_threshold {
            Arc::clone(&large)
        } else {
            Arc::clone(&small)
        };

        let job = Arc::clone(&job);
        let app = app.clone();
        let client = client.clone();

        handles.push(tauri::async_runtime::spawn(async move {
            if job.is_stopped() {
                return;
            }

            let Ok(_permit) = semaphore.acquire().await else {
                return;
            };

            if job.is_stopped() {
                return;
            }

            if let Err(error) = download_one(&app, &client, &job, &task, deep_verify).await {
                job.fail(error);
            }
        }));
    }

    for handle in handles {
        if handle.await.is_err() && !job.cancel.load(Ordering::Relaxed) {
            job.fail(CommandError::download("Задача загрузки аварийно завершилась"));
        }
    }

    let status = if job.cancel.load(Ordering::Relaxed) {
        JobStatus::Cancelled
    } else if job.failed.load(Ordering::Relaxed) {
        let error = job
            .error
            .lock()
            .ok()
            .and_then(|slot| slot.clone())
            .unwrap_or(JobError {
                code: "UNKNOWN".into(),
                message: "Загрузка не удалась".into(),
                details: None,
            });
        JobStatus::Failed { error }
    } else {
        job.done_weight
            .store(job.total_weight, Ordering::Relaxed);
        JobStatus::Finished
    };

    if let Ok(mut active) = job.active.lock() {
        active.clear();
    }

    job.status.send_replace(status);
    job.emit(&app, true);
}

async fn download_one(
    app: &AppHandle,
    client: &reqwest::Client,
    job: &Arc<Job>,
    task: &DownloadTask,
    deep_verify: bool,
) -> Result<(), CommandError> {
    let destination = PathBuf::from(&task.destination);
    let name = destination
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "файл".to_string());

    let weight = if job.use_bytes { task.size.unwrap_or(0) } else { 1 };

    if is_already_valid(&destination, task, deep_verify).await {
        job.add_weight(weight);
        job.done_files.fetch_add(1, Ordering::Relaxed);
        job.skipped_files.fetch_add(1, Ordering::Relaxed);
        job.emit(app, false);
        return Ok(());
    }

    if let Some(parent) = destination.parent() {
        job.ensure_dir(parent).await?;
    }

    let part_path = destination.with_extension(format!(
        "{}part",
        destination
            .extension()
            .map(|e| format!("{}.", e.to_string_lossy()))
            .unwrap_or_default()
    ));

    let mut last_error: Option<CommandError> = None;
    let mut counted: u64 = 0;

    for attempt in 1..=MAX_ATTEMPTS {
        if job.is_stopped() {
            let _ = tokio::fs::remove_file(&part_path).await;
            return Ok(());
        }

        if attempt > 1 {
            tokio::time::sleep(Duration::from_millis(250 * (1 << (attempt - 1)))).await;
        }

        match fetch_to_file(app, client, job, task, &part_path, &name, weight, &mut counted).await {
            Ok(FetchOutcome::Completed) => {
                job.clear_active(&task.url);

                tokio::fs::rename(&part_path, &destination)
                    .await
                    .map_err(|e| {
                        CommandError::fs(format!(
                            "Не удалось сохранить файл: {}",
                            destination.display()
                        ))
                        .with_details(e.to_string())
                    })?;

                job.done_files.fetch_add(1, Ordering::Relaxed);
                job.emit(app, false);
                return Ok(());
            }
            Ok(FetchOutcome::Stopped) => {
                job.clear_active(&task.url);
                let _ = tokio::fs::remove_file(&part_path).await;
                return Ok(());
            }
            Err(error) => {
                job.clear_active(&task.url);
                let _ = tokio::fs::remove_file(&part_path).await;

                let retryable = matches!(error.code, "NETWORK" | "DOWNLOAD_FAILED" | "HASH_MISMATCH");
                last_error = Some(error);

                if !retryable {
                    break;
                }
            }
        }
    }

    Err(last_error.unwrap_or_else(|| {
        CommandError::download(format!("Не удалось скачать {}", task.url))
    }))
}

enum FetchOutcome {
    Completed,
    Stopped,
}

async fn fetch_to_file(
    app: &AppHandle,
    client: &reqwest::Client,
    job: &Arc<Job>,
    task: &DownloadTask,
    part_path: &Path,
    name: &str,
    weight: u64,
    counted: &mut u64,
) -> Result<FetchOutcome, CommandError> {
    let mut response = client.get(&task.url).send().await.map_err(|e| {
        CommandError::network(format!("Не удалось подключиться к {}", task.url))
            .with_details(e.to_string())
    })?;

    let status = response.status();
    if !status.is_success() {
        let message = format!("Сервер ответил HTTP {} на {}", status.as_u16(), task.url);
        return Err(if status.is_server_error() || status.as_u16() == 429 {
            CommandError::network(message)
        } else {
            CommandError::download(message)
        });
    }

    let total = task
        .size
        .or_else(|| response.content_length())
        .unwrap_or(0);

    let mut file = tokio::fs::File::create(part_path).await.map_err(|e| {
        CommandError::fs(format!("Не удалось создать файл: {}", part_path.display()))
            .with_details(e.to_string())
    })?;

    let mut hasher = Sha1::new();
    let mut received: u64 = 0;
    let mut last_percent = -1.0_f64;

    job.set_active(
        &task.url,
        FileProgress {
            url: task.url.clone(),
            name: name.to_string(),
            destination: task.destination.clone(),
            loaded: 0,
            total,
            percent: 0.0,
            done: false,
        },
    );

    loop {
        if job.is_stopped() {
            return Ok(FetchOutcome::Stopped);
        }

        let chunk = response.chunk().await.map_err(|e| {
            CommandError::download(format!("Обрыв загрузки: {}", task.url))
                .with_details(e.to_string())
        })?;

        let Some(chunk) = chunk else { break };

        file.write_all(&chunk).await.map_err(|e| {
            CommandError::fs(format!("Ошибка записи: {}", part_path.display()))
                .with_details(e.to_string())
        })?;

        hasher.update(&chunk);
        received += chunk.len() as u64;
        job.downloaded_bytes
            .fetch_add(chunk.len() as u64, Ordering::Relaxed);

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
            job.set_active(
                &task.url,
                FileProgress {
                    url: task.url.clone(),
                    name: name.to_string(),
                    destination: task.destination.clone(),
                    loaded: received,
                    total,
                    percent,
                    done: false,
                },
            );
        }

        job.emit(app, false);
    }

    file.flush().await.map_err(|e| {
        CommandError::fs(format!("Ошибка записи: {}", part_path.display()))
            .with_details(e.to_string())
    })?;
    drop(file);

    if let (Some("sha1"), Some(expected)) = (task.verification_type.as_deref(), task.hash.as_ref()) {
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

    let has_hash = task.verification_type.as_deref() == Some("sha1") && task.hash.is_some();

    if !deep_verify && (task.size.is_some() || !has_hash) {
        return true;
    }

    if !has_hash {
        return true;
    }

    matches!(file_sha1(destination).await, Some(actual) if Some(&actual) == task.hash.as_ref())
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

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
