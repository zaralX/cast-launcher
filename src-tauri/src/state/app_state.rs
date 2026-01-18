use std::sync::Mutex;
use crate::config::schema::AppConfig;

pub struct AppState {
    pub config: Mutex<AppConfig>,
}