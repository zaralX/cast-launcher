use std::fmt;

#[derive(Debug, serde::Serialize)]
pub struct CommandError {
    pub code: &'static str,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

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

    pub fn from_code(code: &str, message: impl Into<String>) -> Self {
        let code = match code {
            "NETWORK" => "NETWORK",
            "DOWNLOAD_FAILED" => "DOWNLOAD_FAILED",
            "HASH_MISMATCH" => "HASH_MISMATCH",
            "FS_ERROR" => "FS_ERROR",
            "INSTALL_ABORTED" => "INSTALL_ABORTED",
            _ => "UNKNOWN",
        };
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
}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

impl std::error::Error for CommandError {}
