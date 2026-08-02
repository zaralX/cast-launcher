use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

use crate::net::download::{FileProgress, JobSnapshot};
use crate::packs::BlockedFile;

pub type Publisher = Arc<dyn Fn(InstallSnapshot) + Send + Sync>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Stage {
    Prepare,
    Download,
    Install,
    Finalize,
    Finished,
    Aborted,
    Failed,
}

impl Stage {
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Finished | Self::Aborted | Self::Failed)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Phase {
    pub key: &'static str,
    pub label: &'static str,
    pub weight: u32,
}

impl Phase {
    pub const fn new(key: &'static str, label: &'static str, weight: u32) -> Self {
        Self { key, label, weight }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallSnapshot {
    pub instance_id: String,
    pub instance_name: String,
    pub stage: Stage,
    pub phase: String,
    pub message: String,
    pub progress: f64,
    pub files: Vec<FileProgress>,
    pub started_at: u64,
    pub aborting: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub blocked: Vec<BlockedFile>,
    pub awaiting_files: bool,
}

struct State {
    stage: Stage,
    phase: usize,
    message: String,
    progress: f64,
    files: Vec<FileProgress>,
    error: Option<String>,
    blocked: Vec<BlockedFile>,
    awaiting_files: bool,
}

pub struct ProgressReporter {
    publish_to: Publisher,
    instance_id: String,
    instance_name: String,
    phases: Vec<Phase>,
    started_at: u64,
    cancel: AtomicBool,
    downloaded_bytes: AtomicU64,
    phase_bytes: AtomicU64,
    peak_blocked: AtomicU64,
    state: Mutex<State>,
}

impl ProgressReporter {
    pub fn new(
        publish_to: Publisher,
        instance_id: String,
        instance_name: String,
        phases: Vec<Phase>,
    ) -> Self {
        Self {
            publish_to,
            instance_id,
            instance_name,
            phases,
            started_at: now_millis(),
            cancel: AtomicBool::new(false),
            downloaded_bytes: AtomicU64::new(0),
            phase_bytes: AtomicU64::new(0),
            peak_blocked: AtomicU64::new(0),
            state: Mutex::new(State {
                stage: Stage::Prepare,
                phase: 0,
                message: "Подготовка".into(),
                progress: 0.0,
                files: Vec::new(),
                error: None,
                blocked: Vec::new(),
                awaiting_files: false,
            }),
        }
    }

    pub fn instance_id(&self) -> &str {
        &self.instance_id
    }

    pub fn request_cancel(&self) {
        self.cancel.store(true, Ordering::SeqCst);
        self.publish();
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }

    pub fn begin_phase(&self, key: &str, message: &str) {
        let Some(index) = self.phases.iter().position(|phase| phase.key == key) else {
            eprintln!("Неизвестная фаза установки: {key}");
            return;
        };

        {
            let mut state = self.lock();
            state.phase = index;
            state.message = message.to_string();
            state.files.clear();
        }

        self.phase_bytes.store(0, Ordering::SeqCst);
        self.set_fraction(0.0);
    }

    pub fn phase_key(&self) -> &'static str {
        let index = self.lock().phase;

        self.phases.get(index).map(|phase| phase.key).unwrap_or_default()
    }

    pub fn elapsed_seconds(&self) -> f64 {
        now_millis().saturating_sub(self.started_at) as f64 / 1000.0
    }

    pub fn downloaded_bytes(&self) -> u64 {
        self.downloaded_bytes.load(Ordering::Relaxed)
    }

    pub fn set_stage(&self, stage: Stage) {
        self.lock().stage = stage;
        self.publish();
    }

    pub fn set_blocked(&self, blocked: Vec<BlockedFile>) {
        self.peak_blocked.fetch_max(blocked.len() as u64, Ordering::SeqCst);
        self.lock().blocked = blocked;
        self.publish();
    }

    pub fn peak_blocked(&self) -> u64 {
        self.peak_blocked.load(Ordering::Relaxed)
    }

    pub fn set_awaiting_files(&self, awaiting: bool) {
        self.lock().awaiting_files = awaiting;
        self.publish();
    }

    pub fn set_message(&self, message: impl Into<String>) {
        self.lock().message = message.into();
        self.publish();
    }

    pub fn set_fraction(&self, fraction: f64) {
        {
            let mut state = self.lock();
            let overall = self.overall(state.phase, fraction);
            state.progress = state.progress.max(overall);
        }

        self.publish();
    }

    pub fn apply_download(&self, snapshot: &JobSnapshot) {
        let seen = self.phase_bytes.swap(snapshot.downloaded_bytes, Ordering::SeqCst);

        self.downloaded_bytes
            .fetch_add(snapshot.downloaded_bytes.saturating_sub(seen), Ordering::SeqCst);

        {
            let mut state = self.lock();
            state.files = snapshot.files.clone();

            let overall = self.overall(state.phase, snapshot.progress);
            state.progress = state.progress.max(overall);
        }

        self.publish();
    }

    pub fn finish(&self) {
        {
            let mut state = self.lock();
            state.stage = Stage::Finished;
            state.progress = 1.0;
            state.message = "Установка завершена".into();
            state.files.clear();
        }

        self.publish();
    }

    pub fn fail(&self, stage: Stage, message: String) {
        {
            let mut state = self.lock();
            state.stage = stage;
            state.message = message.clone();
            state.error = Some(message);
            state.files.clear();
        }

        self.publish();
    }

    pub fn snapshot(&self) -> InstallSnapshot {
        let state = self.lock();
        let phase = self.phases.get(state.phase);

        InstallSnapshot {
            instance_id: self.instance_id.clone(),
            instance_name: self.instance_name.clone(),
            stage: state.stage,
            phase: phase.map(|phase| phase.label.to_string()).unwrap_or_default(),
            message: state.message.clone(),
            progress: state.progress.clamp(0.0, 1.0),
            files: state.files.clone(),
            started_at: self.started_at,
            aborting: self.is_cancelled() && !state.stage.is_terminal(),
            error: state.error.clone(),
            blocked: state.blocked.clone(),
            awaiting_files: state.awaiting_files && !state.stage.is_terminal(),
        }
    }

    fn publish(&self) {
        (self.publish_to)(self.snapshot());
    }

    fn overall(&self, phase_index: usize, fraction: f64) -> f64 {
        let total: u32 = self.phases.iter().map(|phase| phase.weight).sum();
        if total == 0 {
            return 0.0;
        }

        let before: u32 = self.phases[..phase_index.min(self.phases.len())]
            .iter()
            .map(|phase| phase.weight)
            .sum();

        let current = self.phases.get(phase_index).map(|phase| phase.weight).unwrap_or(0);

        (before as f64 + fraction.clamp(0.0, 1.0) * current as f64) / total as f64
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn phases() -> Vec<Phase> {
        vec![
            Phase::new("java", "Java", 10),
            Phase::new("libraries", "Библиотеки", 30),
            Phase::new("assets", "Ресурсы", 60),
        ]
    }

    fn reporter() -> (Arc<ProgressReporter>, Arc<Mutex<Vec<InstallSnapshot>>>) {
        let published: Arc<Mutex<Vec<InstallSnapshot>>> = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&published);

        let reporter = Arc::new(ProgressReporter::new(
            Arc::new(move |snapshot| sink.lock().unwrap().push(snapshot)),
            "id".into(),
            "Сборка".into(),
            phases(),
        ));

        (reporter, published)
    }

    #[test]
    fn phases_split_the_scale_by_weight() {
        let (reporter, _) = reporter();

        reporter.begin_phase("java", "Java");
        reporter.set_fraction(1.0);
        assert!((reporter.snapshot().progress - 0.1).abs() < 1e-9);

        reporter.begin_phase("libraries", "Библиотеки");
        reporter.set_fraction(0.5);
        assert!((reporter.snapshot().progress - 0.25).abs() < 1e-9);

        reporter.begin_phase("assets", "Ресурсы");
        reporter.set_fraction(1.0);
        assert!((reporter.snapshot().progress - 1.0).abs() < 1e-9);
    }

    #[test]
    fn progress_never_goes_backwards() {
        let (reporter, _) = reporter();

        reporter.begin_phase("libraries", "Библиотеки");
        reporter.set_fraction(1.0);
        let peak = reporter.snapshot().progress;

        reporter.set_fraction(0.0);
        assert_eq!(reporter.snapshot().progress, peak);
    }

    fn job(downloaded_bytes: u64) -> JobSnapshot {
        JobSnapshot {
            job_id: "job".into(),
            status: crate::net::download::JobStatus::Running,
            progress: 0.0,
            total_files: 1,
            done_files: 0,
            skipped_files: 0,
            downloaded_bytes,
            total_bytes: downloaded_bytes,
            files: Vec::new(),
        }
    }

    #[test]
    fn downloaded_bytes_add_up_across_phases() {
        let (reporter, _) = reporter();

        reporter.begin_phase("libraries", "Библиотеки");
        reporter.apply_download(&job(300));
        reporter.apply_download(&job(1000));

        assert_eq!(reporter.downloaded_bytes(), 1000);

        reporter.begin_phase("assets", "Ресурсы");
        reporter.apply_download(&job(500));

        assert_eq!(reporter.downloaded_bytes(), 1500);
    }

    #[test]
    fn the_peak_number_of_blocked_files_is_remembered() {
        let (reporter, _) = reporter();

        let file = |name: &str| BlockedFile {
            file_name: name.into(),
            target_path: format!("mods/{name}"),
            website_url: String::new(),
            sha1: None,
            local_path: None,
        };

        reporter.set_blocked(vec![file("a"), file("b"), file("c")]);
        reporter.set_blocked(vec![file("a")]);
        reporter.set_blocked(Vec::new());

        assert_eq!(reporter.peak_blocked(), 3);
    }

    #[test]
    fn the_current_phase_is_reported_by_its_key() {
        let (reporter, _) = reporter();

        reporter.begin_phase("assets", "Ресурсы");

        assert_eq!(reporter.phase_key(), "assets");
        assert_eq!(reporter.snapshot().phase, "Ресурсы");
    }

    #[test]
    fn unknown_phase_key_is_ignored() {
        let (reporter, _) = reporter();

        reporter.begin_phase("java", "Java");
        reporter.begin_phase("опечатка", "Опечатка");

        assert_eq!(reporter.snapshot().phase, "Java");
    }

    #[test]
    fn every_change_is_published() {
        let (reporter, published) = reporter();

        reporter.begin_phase("java", "Проверка Java");
        reporter.set_message("Скачивание");
        reporter.finish();

        let snapshots = published.lock().unwrap();
        assert!(snapshots.len() >= 3);
        assert_eq!(snapshots.last().unwrap().stage, Stage::Finished);
        assert_eq!(snapshots.last().unwrap().progress, 1.0);
    }

    #[test]
    fn cancellation_shows_up_until_a_terminal_stage() {
        let (reporter, _) = reporter();

        reporter.request_cancel();
        assert!(reporter.snapshot().aborting);

        reporter.fail(Stage::Aborted, "Установка прервана".into());
        assert!(!reporter.snapshot().aborting);
    }

    #[test]
    fn terminal_stages_are_recognised() {
        assert!(Stage::Finished.is_terminal());
        assert!(Stage::Failed.is_terminal());
        assert!(Stage::Aborted.is_terminal());
        assert!(!Stage::Download.is_terminal());
    }
}
