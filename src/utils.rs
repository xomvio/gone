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

/// Compute SHA-256 hash of a file by streaming in chunks (O(1) memory).
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