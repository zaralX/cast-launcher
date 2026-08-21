pub fn decode_line(bytes: &[u8]) -> String {
    let end = bytes
        .iter()
        .rposition(|byte| *byte != b'\n' && *byte != b'\r')
        .map_or(0, |index| index + 1);

    String::from_utf8_lossy(&bytes[..end]).into_owned()
}

#[cfg(test)]
mod tests {
    use super::decode_line;

    #[test]
    fn strips_the_line_ending() {
        assert_eq!(decode_line(b"[main/INFO]: hello\r\n"), "[main/INFO]: hello");
        assert_eq!(decode_line(b"no ending"), "no ending");
        assert_eq!(decode_line(b"\n"), "");
        assert_eq!(decode_line(b""), "");
    }

    #[test]
    fn survives_non_utf8_console_encoding() {
        let line = decode_line(b"Phoenix Colors: Mapping \xa7z to #BF00FF\n");

        assert!(line.starts_with("Phoenix Colors: Mapping "));
        assert!(line.ends_with("z to #BF00FF"));
    }

    const FORGE_STDOUT: &[u8] = b"[modloading-worker-0/INFO]: Registering S2C receiver\n\
         Phoenix Colors: Mapping \xa7z to #BF00FF\n\
         [modloading-worker-0/INFO]: Registering C2S receiver\n";

    #[tokio::test]
    async fn read_loop_reads_past_a_non_utf8_line() {
        use tokio::io::{AsyncBufReadExt, BufReader};

        let mut reader = BufReader::new(FORGE_STDOUT);
        let mut buffer = Vec::new();
        let mut lines = Vec::new();

        loop {
            buffer.clear();

            match reader.read_until(b'\n', &mut buffer).await {
                Ok(0) | Err(_) => break,
                Ok(_) => {}
            }

            lines.push(decode_line(&buffer));
        }

        assert_eq!(lines.len(), 3);
        assert!(lines[2].ends_with("Registering C2S receiver"));
    }

    #[tokio::test]
    async fn next_line_would_have_stopped_at_that_line() {
        use tokio::io::{AsyncBufReadExt, BufReader};

        let mut lines = BufReader::new(FORGE_STDOUT).lines();

        assert!(lines.next_line().await.unwrap().is_some());
        // Старый `while let Ok(Some(line))` молча выходил из цикла вот здесь,
        // дропал ChildStdout и закрывал трубу игре.
        assert!(lines.next_line().await.is_err());
    }
}
