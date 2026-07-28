//! Логика лаунчера, не зависящая от Tauri, и потому проверяемая обычным `cargo test`.
//!
//! Крейт `cast-launcher` поверх добавляет обвязку: команды, события во фронт,
//! реестр состояния и надзор за процессом игры.

pub mod account;
pub mod archive;
pub mod config;
pub mod error;
pub mod events;
pub mod fs_util;
pub mod install;
pub mod instance;
pub mod java;
pub mod launch;
pub mod meta;
pub mod mojang;
pub mod net;
pub mod paths;
