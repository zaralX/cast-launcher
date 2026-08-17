use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Manager};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{oneshot, Mutex, RwLock};
use tokio::task::JoinHandle;

use cast_core::config::AfterLaunch;
use cast_core::error::{CommandError, CommandResult};
use cast_core::instance::Playtime;
use cast_core::launch::args::LaunchCommand;
use cast_core::launch::game::{GameStatus, RunningGame};

use crate::events::{EmitExt, LauncherEvent};
use crate::telemetry::{self, Event};

const LOG_TAIL: usize = 120;

const SETTLE_AFTER: Duration = Duration::from_secs(90);

#[derive(Debug, Default)]
pub struct SpawnOptions {
    pub log_path: Option<PathBuf>,
    pub cleanup_dir: Option<PathBuf>,
    pub after_launch: AfterLaunch,
}

struct Process {
    info: RwLock<RunningGame>,
    kill: Mutex<Option<oneshot::Sender<()>>>,
    tail: Mutex<Vec<String>>,
    log_file: Mutex<Option<tokio::fs::File>>,
    cleanup_dir: Option<PathBuf>,
    after_launch: AfterLaunch,
    settled: AtomicBool,
}

impl Process {
    async fn push_log(&self, line: &str) {
        {
            let mut tail = self.tail.lock().await;
            tail.push(line.to_string());
            if tail.len() > LOG_TAIL {
                tail.remove(0);
            }
        }

        let mut file = self.log_file.lock().await;
        if let Some(file) = file.as_mut() {
            if let Err(error) = file.write_all(format!("{line}\n").as_bytes()).await {
                eprintln!("Не удалось записать лог игры: {error}");
            }
        }
    }

    async fn tail(&self) -> String {
        self.tail.lock().await.join("\n")
    }
}

pub struct LaunchGuard {
    launching: Arc<AtomicUsize>,
}

impl Drop for LaunchGuard {
    fn drop(&mut self) {
        self.launching.fetch_sub(1, Ordering::SeqCst);
    }
}

#[derive(Default)]
pub struct ProcessRegistry {
    processes: RwLock<HashMap<String, Arc<Process>>>,
    launching: Arc<AtomicUsize>,
    alive: AtomicUsize,
}

impl ProcessRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn has_running(&self) -> bool {
        self.alive.load(Ordering::SeqCst) > 0
    }

    pub fn busy(&self) -> bool {
        self.has_running() || self.launching.load(Ordering::SeqCst) > 0
    }

    pub fn claim_launch(&self) -> LaunchGuard {
        self.launching.fetch_add(1, Ordering::SeqCst);

        LaunchGuard {
            launching: Arc::clone(&self.launching),
        }
    }

    pub async fn running(&self) -> Vec<RunningGame> {
        let processes = self.processes.read().await;
        let mut list = Vec::with_capacity(processes.len());

        for process in processes.values() {
            list.push(process.info.read().await.clone());
        }

        list.sort_by_key(|game| game.started_at);
        list
    }

    pub async fn is_running(&self, instance_id: &str) -> bool {
        let processes = self.processes.read().await;

        for process in processes.values() {
            if process.info.read().await.instance_id == instance_id {
                return true;
            }
        }

        false
    }

    pub async fn kill(&self, run_id: &str) -> bool {
        let Some(process) = self.processes.read().await.get(run_id).cloned() else {
            return false;
        };

        let signal = process.kill.lock().await.take();

        match signal {
            Some(signal) => signal.send(()).is_ok(),
            None => false,
        }
    }

    pub async fn kill_instance(&self, instance_id: &str) -> usize {
        let run_ids: Vec<String> = {
            let processes = self.processes.read().await;
            let mut ids = Vec::new();

            for (run_id, process) in processes.iter() {
                if process.info.read().await.instance_id == instance_id {
                    ids.push(run_id.clone());
                }
            }

            ids
        };

        let mut stopped = 0;
        for run_id in run_ids {
            if self.kill(&run_id).await {
                stopped += 1;
            }
        }

        stopped
    }

    pub async fn spawn(
        &self,
        app: AppHandle,
        instance_id: String,
        instance_name: String,
        command: LaunchCommand,
        options: SpawnOptions,
    ) -> CommandResult<RunningGame> {
        let mut child = new_command(&command.java_path)
            .args(&command.args)
            .current_dir(&command.working_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| CommandError::spawn(&command.java_path, e))?;

        let run_id = uuid::Uuid::new_v4().to_string();

        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        let info = RunningGame {
            run_id: run_id.clone(),
            instance_id,
            instance_name,
            pid: child.id(),
            started_at: now_millis(),
            status: GameStatus::Running,
        };

        let (kill, kill_signal) = oneshot::channel();

        let process = Arc::new(Process {
            info: RwLock::new(info.clone()),
            kill: Mutex::new(Some(kill)),
            tail: Mutex::new(Vec::new()),
            log_file: Mutex::new(open_log(options.log_path).await),
            cleanup_dir: options.cleanup_dir,
            after_launch: options.after_launch,
            settled: AtomicBool::new(false),
        });

        self.processes.write().await.insert(run_id.clone(), Arc::clone(&process));
        self.alive.fetch_add(1, Ordering::SeqCst);

        LauncherEvent::GameStarted { game: info.clone() }.emit(&app);

        settle_on_timeout(app.clone(), Arc::clone(&process));

        let pumps = [stdout.map(|out| pump(app.clone(), Arc::clone(&process), out, false)),
                     stderr.map(|err| pump(app.clone(), Arc::clone(&process), err, true))];

        watch(app, run_id, process, child, kill_signal, pumps);

        Ok(info)
    }

    async fn forget(&self, run_id: &str) {
        if self.processes.write().await.remove(run_id).is_some() {
            self.alive.fetch_sub(1, Ordering::SeqCst);
        }
    }
}

fn watch(
    app: AppHandle,
    run_id: String,
    process: Arc<Process>,
    mut child: Child,
    kill_signal: oneshot::Receiver<()>,
    pumps: [Option<JoinHandle<()>>; 2],
) {
    tokio::spawn(async move {
        let (status, stopped) = tokio::select! {
            status = child.wait() => (status.ok(), false),
            _ = kill_signal => {
                let _ = child.kill().await;
                (child.wait().await.ok(), true)
            }
        };

        for pump in pumps.into_iter().flatten() {
            let _ = pump.await;
        }

        let code = status.and_then(|status| status.code());

        let (instance_id, started_at, next) = {
            let mut info = process.info.write().await;
            info.status = if code == Some(0) {
                GameStatus::Exited
            } else {
                GameStatus::Crashed
            };
            (info.instance_id.clone(), info.started_at, info.status)
        };

        LauncherEvent::GameStatus {
            run_id: run_id.clone(),
            instance_id: instance_id.clone(),
            status: next,
        }
        .emit(&app);

        let log_tail = (code != Some(0)).then_some(process.tail().await);

        LauncherEvent::GameExited {
            run_id: run_id.clone(),
            instance_id: instance_id.clone(),
            code,
            log_tail: log_tail.clone(),
        }
        .emit(&app);

        if process.after_launch == AfterLaunch::Hide && process.settled.load(Ordering::SeqCst) {
            crate::window::restore(&app);
        }

        if let Some(dir) = &process.cleanup_dir {
            cast_core::fs_util::remove_dir_if_exists(dir).await;
        }

        if let Some(state) = app.try_state::<Arc<crate::state::AppState>>() {
            track_exit(&app, &state, &instance_id, started_at, code, log_tail.as_deref(), stopped)
                .await;
            record_playtime(&app, &state, &instance_id, started_at).await;
            state.processes.forget(&run_id).await;
        }

        crate::window::exit_if_idle(&app);
    });
}

async fn track_exit(
    app: &AppHandle,
    state: &Arc<crate::state::AppState>,
    instance_id: &str,
    started_at: u64,
    code: Option<i32>,
    log_tail: Option<&str>,
    stopped: bool,
) {
    let session_min = telemetry::minutes(Playtime::session_seconds(started_at, now_millis()));

    let Ok(instance) = state.instances.get(instance_id).await else {
        return;
    };

    let event = match (stopped, log_tail) {
        (true, _) => Event::new("game_stopped").instance(&instance),
        (false, None) => Event::new("game_exited").instance(&instance),
        (false, Some(tail)) => Event::new("game_crashed")
            .instance(&instance)
            .num("exit_code", code.unwrap_or(-1))
            .text("crash", telemetry::classify_crash(tail)),
    };

    telemetry::track(app, event.num("session_min", session_min));
}

async fn record_playtime(
    app: &AppHandle,
    state: &Arc<crate::state::AppState>,
    instance_id: &str,
    started_at: u64,
) {
    let seconds = Playtime::session_seconds(started_at, now_millis());
    let paths = state.paths().await;

    if let Err(error) = state.instances.record_session(&paths, instance_id, seconds).await {
        eprintln!("Не удалось записать наигранное время: {}", error.message);
        return;
    }

    LauncherEvent::Instances {
        instances: state.instances.all().await,
    }
    .emit(app);
}

fn settle(app: &AppHandle, process: &Process) {
    if process.after_launch == AfterLaunch::Nothing {
        return;
    }

    if process.settled.swap(true, Ordering::SeqCst) {
        return;
    }

    crate::window::apply_after_launch(app, process.after_launch);
}

fn game_is_up(line: &str) -> bool {
    let line = line.to_lowercase();

    line.contains("setting user") || line.contains("lwjgl version")
}

fn settle_on_timeout(app: AppHandle, process: Arc<Process>) {
    if process.after_launch == AfterLaunch::Nothing {
        return;
    }

    tokio::spawn(async move {
        tokio::time::sleep(SETTLE_AFTER).await;

        if process.info.read().await.status == GameStatus::Running {
            settle(&app, &process);
        }
    });
}

fn pump<R>(app: AppHandle, process: Arc<Process>, reader: R, is_error: bool) -> JoinHandle<()>
where
    R: tokio::io::AsyncRead + Unpin + Send + 'static,
{
    tokio::spawn(async move {
        let mut lines = BufReader::new(reader).lines();

        while let Ok(Some(line)) = lines.next_line().await {
            process.push_log(&line).await;

            if !process.settled.load(Ordering::SeqCst) && game_is_up(&line) {
                settle(&app, &process);
            }

            let info = process.info.read().await;
            LauncherEvent::GameLog {
                run_id: info.run_id.clone(),
                instance_id: info.instance_id.clone(),
                line,
                is_error,
            }
            .emit(&app);
        }
    })
}

async fn open_log(path: Option<PathBuf>) -> Option<tokio::fs::File> {
    let path = path?;

    if let Some(parent) = path.parent() {
        if let Err(error) = tokio::fs::create_dir_all(parent).await {
            eprintln!("Не удалось создать каталог логов: {error}");
            return None;
        }
    }

    match tokio::fs::File::create(&path).await {
        Ok(file) => Some(file),
        Err(error) => {
            eprintln!("Не удалось создать файл лога {}: {error}", path.display());
            None
        }
    }
}

fn new_command(program: &str) -> Command {
    let mut command = Command::new(program);

    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    command
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
