use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::Serialize;

use crate::error::{CommandError, CommandResult};
use crate::fs_util::ensure_dir;

const EMIT_INTERVAL: Duration = Duration::from_millis(120);

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CopyStats {
    pub files: u64,
    pub bytes: u64,
    pub skipped: u64,
}

impl CopyStats {
    pub fn plus(self, other: Self) -> Self {
        Self {
            files: self.files + other.files,
            bytes: self.bytes + other.bytes,
            skipped: self.skipped + other.skipped,
        }
    }
}

type OnChange<'a> = &'a (dyn Fn(CopyStats) + Send + Sync);
type Cancelled<'a> = &'a (dyn Fn() -> bool + Send + Sync);

pub struct Progress<'a> {
    files: AtomicU64,
    bytes: AtomicU64,
    skipped: AtomicU64,
    on_change: OnChange<'a>,
    cancelled: Cancelled<'a>,
    last_emit: Mutex<Instant>,
}

impl<'a> Progress<'a> {
    pub fn new(on_change: OnChange<'a>, cancelled: Cancelled<'a>) -> Self {
        Self {
            files: AtomicU64::new(0),
            bytes: AtomicU64::new(0),
            skipped: AtomicU64::new(0),
            on_change,
            cancelled,
            last_emit: Mutex::new(Instant::now() - EMIT_INTERVAL),
        }
    }

    pub fn stats(&self) -> CopyStats {
        CopyStats {
            files: self.files.load(Ordering::Relaxed),
            bytes: self.bytes.load(Ordering::Relaxed),
            skipped: self.skipped.load(Ordering::Relaxed),
        }
    }

    pub fn is_cancelled(&self) -> bool {
        (self.cancelled)()
    }

    pub fn flush(&self) {
        if let Ok(mut last) = self.last_emit.lock() {
            *last = Instant::now();
        }

        (self.on_change)(self.stats());
    }

    fn copied(&self, bytes: u64) {
        self.files.fetch_add(1, Ordering::Relaxed);
        self.bytes.fetch_add(bytes, Ordering::Relaxed);
        self.tick();
    }

    fn kept(&self) {
        self.skipped.fetch_add(1, Ordering::Relaxed);
        self.tick();
    }

    fn tick(&self) {
        let ready = match self.last_emit.lock() {
            Ok(mut last) if last.elapsed() >= EMIT_INTERVAL => {
                *last = Instant::now();
                true
            }
            _ => false,
        };

        if ready {
            (self.on_change)(self.stats());
        }
    }
}

fn aborted() -> CommandError {
    CommandError::aborted("Перенос прерван")
}

pub async fn merge_dir(from: &Path, to: &Path, progress: &Progress<'_>) -> CommandResult<()> {
    if !from.is_dir() {
        return Ok(());
    }

    ensure_dir(to).await?;

    let mut entries = tokio::fs::read_dir(from)
        .await
        .map_err(|e| CommandError::io("Не удалось прочитать каталог", from, e))?;

    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|e| CommandError::io("Не удалось прочитать каталог", from, e))?
    {
        if progress.is_cancelled() {
            return Err(aborted());
        }

        let source = entry.path();
        let target = to.join(entry.file_name());

        let Ok(kind) = entry.file_type().await else { continue };

        if kind.is_dir() {
            Box::pin(merge_dir(&source, &target, progress)).await?;
            continue;
        }

        if !kind.is_file() {
            continue;
        }

        copy_file(&source, &target, progress).await?;
    }

    Ok(())
}

pub async fn copy_file(from: &Path, to: &Path, progress: &Progress<'_>) -> CommandResult<bool> {
    if !from.is_file() {
        return Ok(false);
    }

    if to.exists() {
        progress.kept();
        return Ok(false);
    }

    if let Some(parent) = to.parent() {
        ensure_dir(parent).await?;
    }

    let bytes = tokio::fs::copy(from, to)
        .await
        .map_err(|e| CommandError::io("Не удалось скопировать файл", from, e))?;

    progress.copied(bytes);

    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::AtomicUsize;

    fn scratch() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("cast-import-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn never() -> impl Fn() -> bool + Send + Sync {
        || false
    }

    fn silent() -> impl Fn(CopyStats) + Send + Sync {
        |_| {}
    }

    #[test]
    fn stats_add_up_across_stages() {
        let total = CopyStats {
            files: 2,
            bytes: 10,
            skipped: 1,
        }
        .plus(CopyStats {
            files: 3,
            bytes: 5,
            skipped: 4,
        });

        assert_eq!(total.files, 5);
        assert_eq!(total.bytes, 15);
        assert_eq!(total.skipped, 5);
    }

    #[tokio::test]
    async fn existing_files_are_kept_and_counted_separately() {
        let root = scratch();
        let from = root.join("from");
        let to = root.join("to");

        std::fs::create_dir_all(from.join("mods")).unwrap();
        std::fs::create_dir_all(to.join("mods")).unwrap();
        std::fs::write(from.join("mods").join("shared.jar"), "новый").unwrap();
        std::fs::write(to.join("mods").join("shared.jar"), "старый").unwrap();
        std::fs::write(from.join("mods").join("fresh.jar"), "свежий").unwrap();

        let on_change = silent();
        let cancelled = never();
        let progress = Progress::new(&on_change, &cancelled);

        merge_dir(&from, &to, &progress).await.unwrap();

        assert_eq!(
            std::fs::read(to.join("mods").join("shared.jar")).unwrap(),
            "старый".as_bytes()
        );
        assert!(to.join("mods").join("fresh.jar").is_file());

        let stats = progress.stats();
        assert_eq!(stats.files, 1);
        assert_eq!(stats.skipped, 1);
        assert_eq!(stats.bytes, "свежий".len() as u64);

        std::fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn a_missing_source_is_not_an_error() {
        let root = scratch();

        let on_change = silent();
        let cancelled = never();
        let progress = Progress::new(&on_change, &cancelled);

        merge_dir(&root.join("нет"), &root.join("to"), &progress).await.unwrap();

        assert_eq!(progress.stats(), CopyStats::default());

        std::fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn cancelling_stops_the_copy() {
        let root = scratch();
        let from = root.join("from");
        std::fs::create_dir_all(&from).unwrap();

        for index in 0..5 {
            std::fs::write(from.join(format!("{index}.jar")), b"data").unwrap();
        }

        let on_change = silent();
        let cancelled = || true;
        let progress = Progress::new(&on_change, &cancelled);

        let error = merge_dir(&from, &root.join("to"), &progress).await.unwrap_err();

        assert_eq!(error.code, "INSTALL_ABORTED");
        assert_eq!(progress.stats().files, 0);

        std::fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn nested_directories_are_created_on_the_way() {
        let root = scratch();
        let from = root.join("from");
        let deep = from.join("net").join("minecraftforge").join("forge");

        std::fs::create_dir_all(&deep).unwrap();
        std::fs::write(deep.join("forge.jar"), b"jar").unwrap();

        let on_change = silent();
        let cancelled = never();
        let progress = Progress::new(&on_change, &cancelled);

        merge_dir(&from, &root.join("to"), &progress).await.unwrap();

        assert!(root
            .join("to")
            .join("net")
            .join("minecraftforge")
            .join("forge")
            .join("forge.jar")
            .is_file());

        std::fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn progress_is_flushed_on_demand_even_when_throttled() {
        let root = scratch();
        let from = root.join("from");
        std::fs::create_dir_all(&from).unwrap();
        std::fs::write(from.join("a.jar"), b"a").unwrap();

        let calls = AtomicUsize::new(0);
        let on_change = |_: CopyStats| {
            calls.fetch_add(1, Ordering::Relaxed);
        };
        let cancelled = never();
        let progress = Progress::new(&on_change, &cancelled);

        merge_dir(&from, &root.join("to"), &progress).await.unwrap();
        let during = calls.load(Ordering::Relaxed);

        progress.flush();

        assert_eq!(calls.load(Ordering::Relaxed), during + 1);

        std::fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn copying_a_single_file_reports_whether_it_was_written() {
        let root = scratch();
        let source = root.join("client.jar");
        let target = root.join("nested").join("client.jar");
        std::fs::write(&source, b"client").unwrap();

        let on_change = silent();
        let cancelled = never();
        let progress = Progress::new(&on_change, &cancelled);

        assert!(copy_file(&source, &target, &progress).await.unwrap());
        assert!(!copy_file(&source, &target, &progress).await.unwrap());
        assert!(!copy_file(&root.join("нет.jar"), &target, &progress).await.unwrap());

        assert_eq!(progress.stats().files, 1);
        assert_eq!(progress.stats().skipped, 1);

        std::fs::remove_dir_all(&root).ok();
    }
}
