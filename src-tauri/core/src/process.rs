use std::ffi::OsStr;
use std::process::Command;

/// A child process that runs without flashing a console window on Windows.
/// Async callers wrap it in `tokio::process::Command::from`.
pub fn command(program: impl AsRef<OsStr>) -> Command {
    let mut command = Command::new(program);
    hide_console(&mut command);
    command
}

#[cfg(windows)]
fn hide_console(command: &mut Command) {
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
fn hide_console(_command: &mut Command) {}
