use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sha1::{Digest, Sha1};
use tokio::sync::RwLock;
use reqwest::header::{ETAG, IF_MODIFIED_SINCE, IF_NONE_MATCH, LAST_MODIFIED};
use reqwest::StatusCode;

use crate::error::{CommandError, CommandResult};
use crate::fs_util::{read_json_opt, write_atomic, write_json_atomic};
use crate::net::http;

const FRESH_FOR: Duration = Duration::from_secs(15 * 60);

pub struct MetaCache {
    dir: RwLock<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CacheEntry {
    url: String,
    etag: Option<String>,
    last_modified: Option<String>,
    checked_at: u64,
}

impl MetaCache {
    pub fn new(dir: PathBuf) -> Self {
        Self {
            dir: RwLock::new(dir),
        }
    }

    pub async fn relocate(&self, dir: PathBuf) {
        *self.dir.write().await = dir;
    }

    async fn dir(&self) -> PathBuf {
        self.dir.read().await.clone()
    }

    pub async fn fetch_json<T: DeserializeOwned>(&self, url: &str) -> CommandResult<T> {
        self.fetch_json_with(url, &[]).await
    }

    pub async fn fetch_json_with<T: DeserializeOwned>(
        &self,
        url: &str,
        headers: &[(&str, &str)],
    ) -> CommandResult<T> {
        let bytes = self.fetch_bytes_with(url, headers).await?;

        serde_json::from_slice(&bytes).map_err(|e| {
            CommandError::manifest(format!("Некорректный ответ: {url}")).with_details(e.to_string())
        })
    }

    pub async fn fetch_bytes(&self, url: &str) -> CommandResult<Vec<u8>> {
        self.fetch_bytes_with(url, &[]).await
    }

    pub async fn fetch_bytes_with(&self, url: &str, headers: &[(&str, &str)]) -> CommandResult<Vec<u8>> {
        let key = cache_key(url);
        let dir = self.dir().await;
        let body_path = dir.join(format!("{key}.body"));
        let meta_path = dir.join(format!("{key}.json"));

        let entry: Option<CacheEntry> = read_json_opt(&meta_path).await;
        let cached = tokio::fs::read(&body_path).await.ok();

        if let (Some(entry), Some(body)) = (&entry, &cached) {
            if entry.url == url && age(entry.checked_at) < FRESH_FOR {
                return Ok(body.clone());
            }
        }

        let mut request = http::client().get(url);

        for (name, value) in headers {
            request = request.header(*name, *value);
        }

        if cached.is_some() {
            if let Some(entry) = &entry {
                if let Some(etag) = &entry.etag {
                    request = request.header(IF_NONE_MATCH, etag);
                }
                if let Some(last_modified) = &entry.last_modified {
                    request = request.header(IF_MODIFIED_SINCE, last_modified);
                }
            }
        }

        let response = match request.send().await {
            Ok(response) => response,
            Err(error) => {
                return match cached {
                    Some(body) => Ok(body),
                    None => Err(CommandError::network(format!("Не удалось подключиться к {url}"))
                        .with_details(error.to_string())),
                };
            }
        };

        let status = response.status();

        if status == StatusCode::NOT_MODIFIED {
            if let Some(body) = cached {
                self.touch(&meta_path, entry, url, &response).await;
                return Ok(body);
            }
            return self.fetch_uncached(url, headers, &body_path, &meta_path).await;
        }

        if !status.is_success() {
            return match cached {
                Some(body) => Ok(body),
                None => Err(http::http_status_error(status, url)),
            };
        }

        let fresh_entry = CacheEntry {
            url: url.to_string(),
            etag: header(&response, ETAG),
            last_modified: header(&response, LAST_MODIFIED),
            checked_at: now(),
        };

        let body = response
            .bytes()
            .await
            .map_err(|e| {
                CommandError::network(format!("Обрыв ответа: {url}")).with_details(e.to_string())
            })?
            .to_vec();

        self.store(&body_path, &meta_path, &body, &fresh_entry).await;

        Ok(body)
    }

    async fn fetch_uncached(
        &self,
        url: &str,
        headers: &[(&str, &str)],
        body_path: &Path,
        meta_path: &Path,
    ) -> CommandResult<Vec<u8>> {
        let mut request = http::client().get(url);

        for (name, value) in headers {
            request = request.header(*name, *value);
        }

        let response = request.send().await.map_err(|e| {
            CommandError::network(format!("Не удалось подключиться к {url}")).with_details(e.to_string())
        })?;

        let status = response.status();
        if !status.is_success() {
            return Err(http::http_status_error(status, url));
        }

        let entry = CacheEntry {
            url: url.to_string(),
            etag: header(&response, ETAG),
            last_modified: header(&response, LAST_MODIFIED),
            checked_at: now(),
        };

        let body = response
            .bytes()
            .await
            .map_err(|e| CommandError::network(format!("Обрыв ответа: {url}")).with_details(e.to_string()))?
            .to_vec();

        self.store(body_path, meta_path, &body, &entry).await;

        Ok(body)
    }

    async fn touch(
        &self,
        meta_path: &Path,
        entry: Option<CacheEntry>,
        url: &str,
        response: &reqwest::Response,
    ) {
        let entry = CacheEntry {
            url: url.to_string(),
            etag: header(response, ETAG).or_else(|| entry.as_ref().and_then(|e| e.etag.clone())),
            last_modified: header(response, LAST_MODIFIED)
                .or_else(|| entry.as_ref().and_then(|e| e.last_modified.clone())),
            checked_at: now(),
        };

        if let Err(error) = write_json_atomic(meta_path, &entry).await {
            eprintln!("Не удалось обновить метаданные кэша: {error}");
        }
    }

    async fn store(&self, body_path: &Path, meta_path: &Path, body: &[u8], entry: &CacheEntry) {
        if let Err(error) = write_atomic(body_path, body).await {
            eprintln!("Не удалось сохранить кэш манифеста: {error}");
            return;
        }

        if let Err(error) = write_json_atomic(meta_path, entry).await {
            eprintln!("Не удалось сохранить метаданные кэша: {error}");
        }
    }
}

fn header(response: &reqwest::Response, name: impl AsHeaderName) -> Option<String> {
    response
        .headers()
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string)
}

use reqwest::header::AsHeaderName;

fn cache_key(url: &str) -> String {
    let digest = Sha1::digest(url.as_bytes());
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn age(checked_at: u64) -> Duration {
    Duration::from_secs(now().saturating_sub(checked_at))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_keys_are_stable_and_distinct() {
        let a = cache_key("https://example.com/a.json");
        let b = cache_key("https://example.com/b.json");

        assert_eq!(a, cache_key("https://example.com/a.json"));
        assert_ne!(a, b);
        assert_eq!(a.len(), 40);
    }

    #[test]
    fn fresh_window_is_measured_from_last_check() {
        assert!(age(now()) < FRESH_FOR);
        assert!(age(now().saturating_sub(FRESH_FOR.as_secs() + 1)) >= FRESH_FOR);
    }
}
