use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum GameStatus {
    Starting,
    Running,
    Exited,
    Crashed,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunningGame {
    pub run_id: String,
    pub instance_id: String,
    pub instance_name: String,
    pub pid: Option<u32>,
    pub started_at: u64,
    pub status: GameStatus,
}
