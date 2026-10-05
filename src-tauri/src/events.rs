use tauri::{AppHandle, Emitter};

pub use cast_core::events::{LauncherEvent, LAUNCHER_EVENT};

pub trait EmitExt {
    fn emit(self, app: &AppHandle);
}

impl EmitExt for LauncherEvent {
    fn emit(self, app: &AppHandle) {
        if let Err(error) = app.emit(LAUNCHER_EVENT, self) {
            log::error!("Failed to send an event to the frontend: {error}");
        }
    }
}
