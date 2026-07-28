use std::fmt;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct CommandError {
    pub code: &'static str,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

pub type CommandResult<T> = Result<T, CommandError>;

impl CommandError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            details: None,
        }
    }

    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }

    pub fn fs(message: impl Into<String>) -> Self {
        Self::new("FS_ERROR", message)
    }

    pub fn archive(message: impl Into<String>) -> Self {
        Self::new("ARCHIVE_INVALID", message)
    }

    pub fn manifest(message: impl Into<String>) -> Self {
        Self::new("MANIFEST_INVALID", message)
    }

    pub fn version_not_found(message: impl Into<String>) -> Self {
        Self::new("VERSION_NOT_FOUND", message)
    }

    pub fn java_not_found(message: impl Into<String>) -> Self {
        Self::new("JAVA_NOT_FOUND", message)
    }

    pub fn launch(message: impl Into<String>) -> Self {
        Self::new("LAUNCH_FAILED", message)
    }

    pub fn forge(message: impl Into<String>) -> Self {
        Self::new("FORGE_INSTALL_FAILED", message)
    }

    pub fn auth(message: impl Into<String>) -> Self {
        Self::new("AUTH_FAILED", message)
    }

    pub fn auth_expired(message: impl Into<String>) -> Self {
        Self::new("AUTH_EXPIRED", message)
    }

    pub fn no_account(message: impl Into<String>) -> Self {
        Self::new("NO_ACCOUNT", message)
    }

    pub fn port_busy(message: impl Into<String>) -> Self {
        Self::new("AUTH_PORT_BUSY", message)
    }

    pub fn network(message: impl Into<String>) -> Self {
        Self::new("NETWORK", message)
    }

    pub fn download(message: impl Into<String>) -> Self {
        Self::new("DOWNLOAD_FAILED", message)
    }

    pub fn hash_mismatch(message: impl Into<String>) -> Self {
        Self::new("HASH_MISMATCH", message)
    }

    pub fn aborted(message: impl Into<String>) -> Self {
        Self::new("INSTALL_ABORTED", message)
    }

    pub fn unknown(message: impl Into<String>) -> Self {
        Self::new("UNKNOWN", message)
    }

    pub fn is_aborted(&self) -> bool {
        self.code == "INSTALL_ABORTED"
    }

    pub fn from_code(code: &str, message: impl Into<String>) -> Self {
        const KNOWN: &[&str] = &[
            "NETWORK",
            "DOWNLOAD_FAILED",
            "HASH_MISMATCH",
            "FS_ERROR",
            "ARCHIVE_INVALID",
            "MANIFEST_INVALID",
            "VERSION_NOT_FOUND",
            "JAVA_NOT_FOUND",
            "LAUNCH_FAILED",
            "FORGE_INSTALL_FAILED",
            "AUTH_FAILED",
            "AUTH_PORT_BUSY",
            "AUTH_EXPIRED",
            "NO_ACCOUNT",
            "CONFIG_ERROR",
            "UPDATE_FAILED",
            "INSTALL_ABORTED",
        ];

        let code = KNOWN.iter().copied().find(|known| *known == code).unwrap_or("UNKNOWN");
        Self::new(code, message)
    }

    pub fn spawn(program: &str, error: std::io::Error) -> Self {
        if error.kind() == std::io::ErrorKind::NotFound {
            Self::java_not_found(format!("Исполняемый файл не найден: {program}"))
                .with_details(error.to_string())
        } else {
            Self::launch(format!("Не удалось запустить процесс: {program}"))
                .with_details(error.to_string())
        }
    }

    pub fn io(message: impl Into<String>, path: &Path, error: std::io::Error) -> Self {
        Self::fs(message).with_details(format!("{}\n{error}", path.display()))
    }

    pub fn task_panicked(what: &str, error: tokio::task::JoinError) -> Self {
        Self::unknown(format!("Задача прервана: {what}")).with_details(error.to_string())
    }
}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)?;
        if let Some(details) = &self.details {
            write!(f, "\n{details}")?;
        }
        Ok(())
    }
}

impl std::error::Error for CommandError {}
