use std::fmt;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::text::UiText;

/// The category of a failure. The frontend picks the title, icon and generic hint by it,
/// so it must name the cause, not the place where the error happened.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    Network,
    DownloadFailed,
    HashMismatch,
    FsError,
    ArchiveInvalid,
    ManifestInvalid,
    VersionNotFound,
    JavaNotFound,
    LaunchFailed,
    ForgeInstallFailed,
    AuthFailed,
    AuthPortBusy,
    AuthExpired,
    NoAccount,
    ConfigError,
    UpdateFailed,
    InstallAborted,
    InvalidInput,
    Conflict,
    NotFound,
    Unsupported,
    #[serde(other)]
    Unknown,
}

impl ErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Network => "NETWORK",
            Self::DownloadFailed => "DOWNLOAD_FAILED",
            Self::HashMismatch => "HASH_MISMATCH",
            Self::FsError => "FS_ERROR",
            Self::ArchiveInvalid => "ARCHIVE_INVALID",
            Self::ManifestInvalid => "MANIFEST_INVALID",
            Self::VersionNotFound => "VERSION_NOT_FOUND",
            Self::JavaNotFound => "JAVA_NOT_FOUND",
            Self::LaunchFailed => "LAUNCH_FAILED",
            Self::ForgeInstallFailed => "FORGE_INSTALL_FAILED",
            Self::AuthFailed => "AUTH_FAILED",
            Self::AuthPortBusy => "AUTH_PORT_BUSY",
            Self::AuthExpired => "AUTH_EXPIRED",
            Self::NoAccount => "NO_ACCOUNT",
            Self::ConfigError => "CONFIG_ERROR",
            Self::UpdateFailed => "UPDATE_FAILED",
            Self::InstallAborted => "INSTALL_ABORTED",
            Self::InvalidInput => "INVALID_INPUT",
            Self::Conflict => "CONFLICT",
            Self::NotFound => "NOT_FOUND",
            Self::Unsupported => "UNSUPPORTED",
            Self::Unknown => "UNKNOWN",
        }
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    pub code: ErrorCode,
    /// What happened, as an i18n key with parameters; the frontend translates it.
    pub text: UiText,
    /// Technical data for reports and logs: paths, library errors, HTTP bodies.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

pub type CommandResult<T> = Result<T, CommandError>;

pub fn error_chain(error: &dyn std::error::Error) -> String {
    let mut chain = error.to_string();
    let mut source = error.source();

    while let Some(cause) = source {
        let text = cause.to_string();
        if !chain.ends_with(&text) {
            chain.push_str(": ");
            chain.push_str(&text);
        }
        source = cause.source();
    }

    chain
}

impl CommandError {
    pub fn new(code: ErrorCode, key: &str) -> Self {
        Self::from_text(code, UiText::new(key))
    }

    pub fn from_text(code: ErrorCode, text: UiText) -> Self {
        Self {
            code,
            text,
            details: None,
        }
    }

    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }

    pub fn param(mut self, name: &str, value: impl fmt::Display) -> Self {
        self.text = self.text.param(name, value);
        self
    }

    pub fn param_text(mut self, name: &str, value: UiText) -> Self {
        self.text = self.text.param_text(name, value);
        self
    }

    pub fn invalid_input(key: &str) -> Self {
        Self::new(ErrorCode::InvalidInput, key)
    }

    pub fn conflict(key: &str) -> Self {
        Self::new(ErrorCode::Conflict, key)
    }

    pub fn not_found(key: &str) -> Self {
        Self::new(ErrorCode::NotFound, key)
    }

    pub fn unsupported(key: &str) -> Self {
        Self::new(ErrorCode::Unsupported, key)
    }

    pub fn fs(key: &str) -> Self {
        Self::new(ErrorCode::FsError, key)
    }

    pub fn archive(key: &str) -> Self {
        Self::new(ErrorCode::ArchiveInvalid, key)
    }

    pub fn manifest(key: &str) -> Self {
        Self::new(ErrorCode::ManifestInvalid, key)
    }

    pub fn version_not_found(key: &str) -> Self {
        Self::new(ErrorCode::VersionNotFound, key)
    }

    pub fn java_not_found(key: &str) -> Self {
        Self::new(ErrorCode::JavaNotFound, key)
    }

    pub fn launch(key: &str) -> Self {
        Self::new(ErrorCode::LaunchFailed, key)
    }

    pub fn forge(key: &str) -> Self {
        Self::new(ErrorCode::ForgeInstallFailed, key)
    }

    pub fn auth(key: &str) -> Self {
        Self::new(ErrorCode::AuthFailed, key)
    }

    pub fn auth_expired(key: &str) -> Self {
        Self::new(ErrorCode::AuthExpired, key)
    }

    pub fn no_account(key: &str) -> Self {
        Self::new(ErrorCode::NoAccount, key)
    }

    pub fn port_busy(key: &str) -> Self {
        Self::new(ErrorCode::AuthPortBusy, key)
    }

    pub fn network(key: &str) -> Self {
        Self::new(ErrorCode::Network, key)
    }

    pub fn download(key: &str) -> Self {
        Self::new(ErrorCode::DownloadFailed, key)
    }

    pub fn hash_mismatch(key: &str) -> Self {
        Self::new(ErrorCode::HashMismatch, key)
    }

    pub fn aborted(key: &str) -> Self {
        Self::new(ErrorCode::InstallAborted, key)
    }

    pub fn unknown(key: &str) -> Self {
        Self::new(ErrorCode::Unknown, key)
    }

    pub fn is_aborted(&self) -> bool {
        self.code == ErrorCode::InstallAborted
    }

    pub fn spawn(program: &str, error: std::io::Error) -> Self {
        let error_text = error.to_string();

        match error.kind() {
            std::io::ErrorKind::NotFound => {
                Self::java_not_found("error.reason.launch.executable_not_found")
            }
            _ => Self::launch("error.reason.launch.process_failed"),
        }
        .param("program", program)
        .with_details(error_text)
    }

    /// A filesystem error; `path` becomes the `{path}` parameter of the text.
    pub fn io(key: &str, path: &Path, error: std::io::Error) -> Self {
        Self::fs(key)
            .param("path", path.display())
            .with_details(error.to_string())
    }

    /// `task` is an identifier for the details, not text for the user.
    pub fn task_panicked(task: &str, error: tokio::task::JoinError) -> Self {
        Self::unknown("error.reason.task_failed").with_details(format!("{task}: {error}"))
    }
}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.code, self.text)?;
        if let Some(details) = &self.details {
            write!(f, "\n{details}")?;
        }
        Ok(())
    }
}

impl std::error::Error for CommandError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt;

    #[derive(Debug)]
    struct Layer {
        text: &'static str,
        cause: Option<Box<Layer>>,
    }

    impl fmt::Display for Layer {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str(self.text)
        }
    }

    impl std::error::Error for Layer {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            self.cause
                .as_deref()
                .map(|cause| cause as &dyn std::error::Error)
        }
    }

    fn layer(text: &'static str, cause: Option<Layer>) -> Layer {
        Layer {
            text,
            cause: cause.map(Box::new),
        }
    }

    #[test]
    fn the_real_cause_is_at_the_bottom_of_the_chain() {
        let error = layer(
            "error sending request for url (https://example.com)",
            Some(layer(
                "client error (Connect)",
                Some(layer("tcp connect error", None)),
            )),
        );

        assert_eq!(
            error_chain(&error),
            "error sending request for url (https://example.com): \
             client error (Connect): tcp connect error"
        );
    }

    #[test]
    fn an_error_without_a_cause_reads_the_same_as_before() {
        let error = layer("no space left on device", None);

        assert_eq!(error_chain(&error), "no space left on device");
    }

    #[test]
    fn a_layer_that_only_repeats_its_cause_is_not_printed_twice() {
        let error = layer("nested cause", Some(layer("nested cause", None)));

        assert_eq!(error_chain(&error), "nested cause");
    }

    #[test]
    fn the_frontend_gets_a_code_a_key_and_details_but_no_prose() {
        let error =
            CommandError::invalid_input("error.reason.instance.name_empty").with_details("trace");

        assert_eq!(
            serde_json::to_value(&error).unwrap(),
            serde_json::json!({
                "code": "INVALID_INPUT",
                "text": { "key": "error.reason.instance.name_empty" },
                "details": "trace",
            })
        );
        assert_eq!(
            serde_json::to_value(CommandError::fs("error.reason.fs.read_file")).unwrap()["code"],
            "FS_ERROR"
        );
    }

    #[test]
    fn an_io_error_names_the_path_in_the_text_and_keeps_the_os_error_in_details() {
        let error = CommandError::io(
            "error.reason.fs.read_file",
            Path::new("instances/abc/instance.json"),
            std::io::Error::other("disk on fire"),
        );

        assert_eq!(error.code, ErrorCode::FsError);
        assert!(error.text.mentions("instance.json"));
        assert_eq!(error.details.as_deref(), Some("disk on fire"));
        assert!(error
            .to_string()
            .starts_with("[FS_ERROR] error.reason.fs.read_file {path: "));
    }

    #[test]
    fn an_unknown_code_from_storage_becomes_unknown() {
        let error: CommandError = serde_json::from_value(
            serde_json::json!({ "code": "SOMETHING_NEW", "text": { "key": "k" } }),
        )
        .unwrap();

        assert_eq!(error.code, ErrorCode::Unknown);
    }

    #[test]
    fn as_str_matches_the_serialized_name() {
        for code in [
            ErrorCode::Network,
            ErrorCode::FsError,
            ErrorCode::InstallAborted,
            ErrorCode::InvalidInput,
            ErrorCode::NotFound,
            ErrorCode::Unknown,
        ] {
            assert_eq!(serde_json::to_value(code).unwrap(), code.as_str());
        }
    }
}
