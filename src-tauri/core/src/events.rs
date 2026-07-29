use serde::Serialize;

use crate::import::ImportProgress;
use crate::install::progress::InstallSnapshot;
use crate::instance::Instance;
use crate::launch::game::{GameStatus, RunningGame};

pub const LAUNCHER_EVENT: &str = "launcher://event";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase", tag = "type")]
pub enum LauncherEvent {
    Install(InstallSnapshot),
    Import(ImportProgress),
    Instances { instances: Vec<Instance> },
    GameStarted {
        game: RunningGame,
    },
    GameStatus {
        run_id: String,
        instance_id: String,
        status: GameStatus,
    },
    GameLog {
        run_id: String,
        instance_id: String,
        line: String,
        is_error: bool,
    },
    GameExited {
        run_id: String,
        instance_id: String,
        code: Option<i32>,
        log_tail: Option<String>,
    },
    LaunchFailed {
        instance_id: String,
        instance_name: String,
        error: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::install::progress::Stage;

    fn game() -> RunningGame {
        RunningGame {
            run_id: "run".into(),
            instance_id: "instance".into(),
            instance_name: "Сборка".into(),
            pid: Some(42),
            started_at: 1,
            status: GameStatus::Running,
        }
    }

    fn wire(event: LauncherEvent) -> serde_json::Value {
        serde_json::to_value(event).unwrap()
    }

    #[test]
    fn variant_names_are_camel_case() {
        assert_eq!(wire(LauncherEvent::GameStarted { game: game() })["type"], "gameStarted");
        assert_eq!(wire(LauncherEvent::Instances { instances: Vec::new() })["type"], "instances");
    }

    #[test]
    fn variant_fields_are_camel_case() {
        let event = wire(LauncherEvent::GameExited {
            run_id: "run".into(),
            instance_id: "instance".into(),
            code: Some(1),
            log_tail: Some("падение".into()),
        });

        assert_eq!(event["type"], "gameExited");
        assert_eq!(event["runId"], "run");
        assert_eq!(event["instanceId"], "instance");
        assert_eq!(event["code"], 1);
        assert_eq!(event["logTail"], "падение");
        assert!(event.get("run_id").is_none(), "snake_case на фронт уходить не должен");
    }

    #[test]
    fn game_log_carries_the_error_flag() {
        let event = wire(LauncherEvent::GameLog {
            run_id: "run".into(),
            instance_id: "instance".into(),
            line: "hello".into(),
            is_error: true,
        });

        assert_eq!(event["isError"], true);
        assert_eq!(event["line"], "hello");
    }

    #[test]
    fn game_status_is_lowercase() {
        let event = wire(LauncherEvent::GameStatus {
            run_id: "run".into(),
            instance_id: "instance".into(),
            status: GameStatus::Crashed,
        });

        assert_eq!(event["status"], "crashed");
    }

    #[test]
    fn nested_structs_keep_their_own_camel_case() {
        let event = wire(LauncherEvent::GameStarted { game: game() });

        assert_eq!(event["game"]["runId"], "run");
        assert_eq!(event["game"]["instanceName"], "Сборка");
        assert_eq!(event["game"]["startedAt"], 1);
    }

    #[test]
    fn install_snapshot_is_flattened_into_the_event() {
        let snapshot = InstallSnapshot {
            instance_id: "instance".into(),
            instance_name: "Сборка".into(),
            stage: Stage::Download,
            phase: "Ресурсы".into(),
            message: "Загрузка".into(),
            progress: 0.5,
            files: Vec::new(),
            started_at: 1,
            aborting: false,
            error: None,
            blocked: Vec::new(),
            awaiting_files: false,
        };

        let event = wire(LauncherEvent::Install(snapshot));

        assert_eq!(event["type"], "install");
        assert_eq!(event["instanceId"], "instance");
        assert_eq!(event["startedAt"], 1);
        assert_eq!(event["stage"], "download");
        assert!(event.get("blocked").is_none(), "пустой список не занимает место в событии");
    }

    #[test]
    fn a_failed_autolaunch_names_the_instance() {
        let event = wire(LauncherEvent::LaunchFailed {
            instance_id: "instance".into(),
            instance_name: "Сборка".into(),
            error: "Java не найдена".into(),
        });

        assert_eq!(event["type"], "launchFailed");
        assert_eq!(event["instanceId"], "instance");
        assert_eq!(event["instanceName"], "Сборка");
        assert_eq!(event["error"], "Java не найдена");
    }

    #[test]
    fn import_progress_is_flattened_into_the_event() {
        let event = wire(LauncherEvent::Import(ImportProgress {
            source: crate::import::LauncherKind::Prism,
            stage: crate::import::ImportStage::Instances,
            step: "Fabulously Optimized".into(),
            done: 2,
            total: 5,
            stats: Default::default(),
        }));

        assert_eq!(event["type"], "import");
        assert_eq!(event["stage"], "instances");
        assert_eq!(event["step"], "Fabulously Optimized");
        assert_eq!(event["total"], 5);
    }
}
