use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Read, Write};
use rand::distr::Alphanumeric;
use rand::Rng;
use chrono;

use crate::config::Config;
use crate::visitor::Visit;


pub fn random_port() -> u16 {
    let mut rng = rand::rng();
    rng.random_range(1024..=65535)
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

pub fn cow_str_to_str<'a>(cow: &'a Option<std::borrow::Cow<'static, str>>, default: &'static str) -> &'a str {
    cow.as_deref().unwrap_or(default)
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
        if buf.len() > 16_384 {
            return None;
        }
    }
    String::from_utf8(buf).ok()
}


pub fn open_log_file(config: &Config) -> Option<BufWriter<File>> {
    config.server.output.as_ref().map(|path| {
        OpenOptions::new().create(true).append(true).open(path)
            .unwrap_or_else(|e| {
                eprintln!("Failed to open log file '{}': {}", path, e);
                std::process::exit(1);
            })
    }).map(BufWriter::new)
}

pub fn log_request(visit: &Visit, status: &str, log_file: &mut Option<BufWriter<File>>) {
    let log = format!(
        "Request\nDateTime: {}\nIP: {}\nEndpoint: {}\nMethod: {}\nVersion: {}\n{}",
        visit.datetime, visit.ip, visit.endpoint, visit.method, visit.version,
        if status.is_empty() { String::new() } else { format!("{status}\n") }
    );

    println!("{}", log);

    if let Some(f) = log_file {
        let _ = writeln!(f, "{}", log);
    }
}