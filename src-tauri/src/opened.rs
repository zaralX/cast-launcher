use std::path::Path;
use std::sync::Arc;

use tauri::{AppHandle, Manager};

use cast_core::castpack::file::opened_paths;

use crate::events::{EmitExt, LauncherEvent};
use crate::state::AppState;

/// Files from the command line of the first start: the window takes them after `bootstrap`.
pub fn at_start(state: &AppState) {
    let args: Vec<String> = std::env::args().collect();
    let cwd = std::env::current_dir().ok();

    state.keep_opened(opened_paths(&args, cwd.as_deref()));
}

/// Files from a second start of the launcher, which hands its arguments over and quits.
pub fn from_second_start(app: &AppHandle, args: &[String], cwd: &str) {
    receive(app, opened_paths(args, Some(Path::new(cwd))));
}

pub fn receive(app: &AppHandle, files: Vec<String>) {
    if files.is_empty() {
        return;
    }

    log::info!("The system asked to open {} pack file(s)", files.len());

    let Some(state) = app.try_state::<Arc<AppState>>() else {
        log::warn!("Pack files arrived before the launcher was ready, ignoring them");
        return;
    };

    state.keep_opened(files);

    LauncherEvent::FilesOpened.emit(app);
}
