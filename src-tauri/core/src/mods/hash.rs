use std::path::Path;

use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use tokio::io::AsyncReadExt;

const MAX_IN_MEMORY: u64 = 64 * 1024 * 1024;

const CHUNK: usize = 128 * 1024;

const SKIPPED: [u8; 4] = [b'\t', b'\n', b'\r', b' '];

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct FileHashes {
    pub sha1: String,
    pub fingerprint: Option<u32>,
}

pub async fn of(path: &Path) -> Option<FileHashes> {
    let size = tokio::fs::metadata(path).await.ok()?.len();

    if size > MAX_IN_MEMORY {
        return Some(FileHashes {
            sha1: streamed_sha1(path).await?,
            fingerprint: None,
        });
    }

    let bytes = tokio::fs::read(path).await.ok()?;

    Some(FileHashes {
        sha1: hex(&Sha1::digest(&bytes)),
        fingerprint: Some(murmur2(&normalized(&bytes))),
    })
}

/// https://github.com/aappleby/smhasher
pub fn murmur2(bytes: &[u8]) -> u32 {
    const M: u32 = 0x5bd1_e995;
    const R: u32 = 24;

    let mut hash = 1_u32 ^ (bytes.len() as u32);
    let mut chunks = bytes.chunks_exact(4);

    for chunk in &mut chunks {
        let mut key = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);

        key = key.wrapping_mul(M);
        key ^= key >> R;
        key = key.wrapping_mul(M);

        hash = hash.wrapping_mul(M);
        hash ^= key;
    }

    let tail = chunks.remainder();

    if tail.len() == 3 {
        hash ^= (tail[2] as u32) << 16;
    }

    if tail.len() >= 2 {
        hash ^= (tail[1] as u32) << 8;
    }

    if !tail.is_empty() {
        hash ^= tail[0] as u32;
        hash = hash.wrapping_mul(M);
    }

    hash ^= hash >> 13;
    hash = hash.wrapping_mul(M);
    hash ^ (hash >> 15)
}

pub fn normalized(bytes: &[u8]) -> Vec<u8> {
    bytes.iter().copied().filter(|byte| !SKIPPED.contains(byte)).collect()
}

async fn streamed_sha1(path: &Path) -> Option<String> {
    let mut file = tokio::fs::File::open(path).await.ok()?;
    let mut hasher = Sha1::new();
    let mut buffer = vec![0_u8; CHUNK];

    loop {
        let read = file.read(&mut buffer).await.ok()?;

        if read == 0 {
            break;
        }

        hasher.update(&buffer[..read]);
    }

    Some(hex(&hasher.finalize()))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fingerprint_matches_the_reference_implementation() {
        assert_eq!(murmur2(&normalized(b"hello world")), 2_824_650_221);
        assert_eq!(murmur2(&normalized(b"")), 1_540_447_798);
        assert_eq!(murmur2(&normalized(b"jar")), 2_164_523_432);
        assert_eq!(murmur2(&normalized(b"cast-launcher mods")), 1_247_997_552);
    }

    #[test]
    fn whitespace_does_not_change_the_fingerprint() {
        assert_eq!(
            murmur2(&normalized(b"hello world")),
            murmur2(&normalized(b"hello\tworld\r\n"))
        );
        assert_ne!(murmur2(&normalized(b"hello")), murmur2(&normalized(b"hellp")));
    }

    #[tokio::test]
    async fn a_file_gives_both_hashes_in_one_read() {
        let dir = std::env::temp_dir().join(format!("cast-hash-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("mod.jar");
        std::fs::write(&file, b"hello world").unwrap();

        let hashes = of(&file).await.unwrap();

        assert_eq!(hashes.sha1, "2aae6c35c94fcfb415dbe95f408b9ce91ee846ed");
        assert_eq!(hashes.fingerprint, Some(2_824_650_221));

        assert!(of(&dir.join("nope.jar")).await.is_none());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn the_streamed_and_the_in_memory_sha1_agree() {
        let dir = std::env::temp_dir().join(format!("cast-hash-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("mod.jar");

        let bytes: Vec<u8> = (0..(CHUNK * 2 + 17)).map(|index| (index % 251) as u8).collect();
        std::fs::write(&file, &bytes).unwrap();

        assert_eq!(streamed_sha1(&file).await.unwrap(), hex(&Sha1::digest(&bytes)));

        std::fs::remove_dir_all(&dir).ok();
    }
}
