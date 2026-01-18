use crate::config::schema::AppConfig;
use std::sync::Mutex;

pub struct AppState {
    pub config: Mutex<AppConfig>,
}
