use std::sync::OnceLock;
use std::time::Duration;

use reqwest;

use crate::error::CommandError;

pub const SMALL_CONCURRENCY: usize = 24;
pub const LARGE_CONCURRENCY: usize = 4;

static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

pub fn client() -> &'static reqwest::Client {
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .pool_max_idle_per_host(SMALL_CONCURRENCY * 2)
            .pool_idle_timeout(Duration::from_secs(90))
            .connect_timeout(Duration::from_secs(15))
            .user_agent(user_agent())
            .build()
            .unwrap_or_else(|error| {
                log::warn!("Failed to build the HTTP client, using the default one: {error}");
                reqwest::Client::new()
            })
    })
}

fn user_agent() -> String {
    format!("cast-launcher/{}", env!("CARGO_PKG_VERSION"))
}

pub fn http_status_error(status: reqwest::StatusCode, url: &str) -> CommandError {
    let key = "error.reason.network.http_status";

    let error = if status.is_server_error() || status.as_u16() == 429 {
        CommandError::network(key)
    } else {
        CommandError::download(key)
    };

    error.param("status", status.as_u16()).param("url", url)
}
