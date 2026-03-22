use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Read, Write};
use rand::distr::Alphanumeric;
use rand::Rng;
use sha2::{Sha256, Digest};
use crate::config::Config;
use crate::constants;
use crate::visitor::Visit;


pub fn random_port() -> u16 {
    let mut rng = rand::rng();
    rng.random_range(constants::MIN_PORT..=65535)
}

pub fn random_endpoint() -> String {
    let rng = rand::rng();
    rng.sample_iter(&Alphanumeric)
        .take(64)
        .map(char::from)
        .collect()
}

pub fn now_str() -> String {
    chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

/// Reads raw HTTP request bytes until `\r\n\r\n` is found (end of headers).
/// Returns None on connection close or if the request exceeds 16 KB.
pub fn read_request<R: Read>(stream: &mut R) -> Option<String> {
    let mut buf = Vec::new();
    let mut tmp = [0u8; 4096];
    loop {
        let n = stream.read(&mut tmp).ok()?;
        if n == 0 {
            return None;
        }
        buf.extend_from_slice(&tmp[..n]);
        if buf.windows(4).any(|w| w == b"\r\n\r\n") {
            break;
        }
        if buf.len() > constants::MAX_REQUEST_SIZE {
            return None;
        }
    }
    String::from_utf8(buf).ok()
}


pub fn open_log_file(config: &Config) -> Result<Option<BufWriter<File>>, String> {
    match &config.server.output {
        Some(path) => {
            let file = OpenOptions::new().create(true).append(true).open(path)
                .map_err(|e| format!("Failed to open log file '{}': {}", path, e))?;
            Ok(Some(BufWriter::new(file)))
        }
        None => Ok(None),
    }
}

pub fn log_request(visit: &Visit, status: &str, log_file: &mut Option<BufWriter<File>>, quiet: bool) {
    let log = format!(
        "Request\nDateTime: {}\nIP: {}\nEndpoint: {}\nMethod: {}\nVersion: {}\n{}",
        visit.datetime, visit.ip, visit.endpoint, visit.method, visit.version,
        if status.is_empty() { String::new() } else { format!("{status}\n") }
    );

    if !quiet {
        println!("{}", log);
    }

    if let Some(f) = log_file {
        let _ = writeln!(f, "{}", log);
    }
}

// Generate SHA-256 hash for a file by streaming in chunks (for big files)
pub fn sha256_file(path: &str) -> Option<String> {
    let mut file = File::open(path).ok()?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = file.read(&mut buf).ok()?;
        if n == 0 { break; }
        hasher.update(&buf[..n]);
    }
    Some(format!("{:x}", hasher.finalize()))
}

/// Compute SHA-256 hash of a text string.
pub fn sha256_text(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    format!("{:x}", hasher.finalize())
}


// Tests __________________________________________
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn sha256_text_known_value() {
        // echo -n "hello" | sha256sum
        assert_eq!(
            sha256_text("hello"),
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }

    #[test]
    fn sha256_text_empty() {
        // echo -n "" | sha256sum
        assert_eq!(
            sha256_text(""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn sha256_file_matches_text() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        std::fs::write(&path, "hello").unwrap();
        assert_eq!(
            sha256_file(path.to_str().unwrap()).unwrap(),
            sha256_text("hello")
        );
    }

    #[test]
    fn sha256_file_nonexistent_returns_none() {
        assert!(sha256_file("/nonexistent/file.txt").is_none());
    }

    #[test]
    fn random_endpoint_length() {
        let ep = random_endpoint();
        assert_eq!(ep.len(), 64);
    }

    #[test]
    fn random_endpoint_alphanumeric() {
        let ep = random_endpoint();
        assert!(ep.chars().all(|c| c.is_ascii_alphanumeric()));
    }

    #[test]
    fn read_valid_request() {
        let data = b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let mut cursor = Cursor::new(data.to_vec());
        let result = read_request(&mut cursor);
        assert!(result.is_some());
        assert!(result.unwrap().contains("GET / HTTP/1.1"));
    }

    #[test]
    fn read_empty_stream_returns_none() {
        let mut cursor = Cursor::new(Vec::new());
        assert!(read_request(&mut cursor).is_none());
    }

    #[test]
    fn read_oversized_request_returns_none() {
        // 17KB of 'A' without \r\n\r\n
        let data = vec![b'A'; constants::MAX_REQUEST_SIZE + 1024];
        let mut cursor = Cursor::new(data);
        assert!(read_request(&mut cursor).is_none());
    }

    #[test]
    fn random_port_in_range() {
        for _ in 0..100 {
            let port = random_port();
            assert!(port >= constants::MIN_PORT);
        }
    }
}