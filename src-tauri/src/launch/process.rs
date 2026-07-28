use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Manager};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{oneshot, Mutex, RwLock};
use tokio::task::JoinHandle;

use cast_core::error::{CommandError, CommandResult};
use cast_core::launch::args::LaunchCommand;
use cast_core::launch::game::{GameStatus, RunningGame};

use crate::events::{EmitExt, LauncherEvent};

const LOG_TAIL: usize = 120;

#[derive(Debug, Default)]
pub struct SpawnOptions {
    pub log_path: Option<PathBuf>,
    pub cleanup_dir: Option<PathBuf>,
}

struct Process {
    info: RwLock<RunningGame>,
    kill: Mutex<Option<oneshot::Sender<()>>>,
    tail: Mutex<Vec<String>>,
    log_file: Mutex<Option<tokio::fs::File>>,
    cleanup_dir: Option<PathBuf>,
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

#[derive(Default)]
pub struct ProcessRegistry {
    processes: RwLock<HashMap<String, Arc<Process>>>,
}

impl ProcessRegistry {
    pub fn new() -> Self {
        Self::default()
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
        });

        self.processes.write().await.insert(run_id.clone(), Arc::clone(&process));

        LauncherEvent::GameStarted { game: info.clone() }.emit(&app);

        let pumps = [stdout.map(|out| pump(app.clone(), Arc::clone(&process), out, false)),
                     stderr.map(|err| pump(app.clone(), Arc::clone(&process), err, true))];

        watch(app, run_id, process, child, kill_signal, pumps);

        Ok(info)
    }

    async fn forget(&self, run_id: &str) {
        self.processes.write().await.remove(run_id);
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
        let status = tokio::select! {
            status = child.wait() => status.ok(),
            _ = kill_signal => {
                let _ = child.kill().await;
                child.wait().await.ok()
            }
        };

        for pump in pumps.into_iter().flatten() {
            let _ = pump.await;
        }

        let code = status.and_then(|status| status.code());

        let (instance_id, next) = {
            let mut info = process.info.write().await;
            info.status = if code == Some(0) {
                GameStatus::Exited
            } else {
                GameStatus::Crashed
            };
            (info.instance_id.clone(), info.status)
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
            instance_id,
            code,
            log_tail,
        }
        .emit(&app);

        if let Some(dir) = &process.cleanup_dir {
            cast_core::fs_util::remove_dir_if_exists(dir).await;
        }

        if let Some(state) = app.try_state::<Arc<crate::state::AppState>>() {
            state.processes.forget(&run_id).await;
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
