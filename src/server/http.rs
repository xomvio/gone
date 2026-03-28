use std::fs::File;
use std::io::{Read, Write};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;

use crate::{config::Config, constants};

fn http_date() -> String {
    chrono::Utc::now().format("%a, %d %b %Y %H:%M:%S GMT").to_string()
}

/// Parses the HTTP request line. Returns (method, url, version) or None on malformed input.
/// Example: "GET /endpoint HTTP/1.1\r\nHost: ..." → ("GET", "/endpoint", "HTTP/1.1")
pub fn parse_request_line(raw: &str) -> Option<(String, String, String)> {
    let first_line = raw.lines().next()?;
    let mut parts = first_line.splitn(3, ' ');
    let method  = parts.next()?.to_string();
    let url     = parts.next()?.to_string();
    let version = parts.next()?.trim_end_matches('\r').to_string();
    Some((method, url, version))
}

/// Sends a plain HTTP 404 response.
pub fn send_404<W: Write>(stream: &mut W, server_name: &str) {
    let body = b"404 Not Found";
    let _ = write!(
        stream,
        "HTTP/1.1 404 Not Found\r\nServer: {server_name}\r\nDate: {}\r\nConnection: close\r\nContent-Length: {}\r\n\r\n",
        http_date(), body.len()
    );
    let _ = stream.write_all(body);
}

/// Sends an HTTP 200 response with the configured content.
///
/// Uses `std::io::copy` for file content — no full-file buffering, binary-safe.
/// Returns true on success, false on any read/write error.
pub fn serve_content<W: Write>(stream: &mut W, config: &Config, server_name: &str) -> bool {
    let content_type = config.server.content_type.as_deref().unwrap_or(constants::DEFAULT_CONTENT_TYPE);

    match &config.content.from_file {
        // Handle from_file first
        Some(path) => {
            let path = PathBuf::from(path);
            let file = match File::open(&path) {
                Ok(f) => f,
                Err(e) => {
                    eprintln!("Failed to open file '{}': {}", path.to_string_lossy(), e);
                    return false;
                }
            };
            let size = match file.metadata() {
                Ok(m) => m.len(),
                Err(e) => {
                    eprintln!("Failed to stat file '{}': {}", path.to_string_lossy(), e);
                    return false;
                }
            };
            let attachment_string = format!("Content-Disposition: attachment; filename=\"{}\"\r\n", String::from_utf8_lossy(path.file_name().unwrap().as_bytes()));
            let header = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\n{attachment_string}Server: {server_name}\r\nDate: {}\r\nConnection: close\r\nContent-Length: {size}\r\n\r\n",
                http_date()
            );
            if stream.write_all(header.as_bytes()).is_err() {
                return false;
            }
            send_file(file, stream, config.server.quiet.unwrap_or(false)).is_ok()
        }
        // if there is from_file is None, then check if stdin is used.
        None => match &config.content.stdin_data {
            Some(data) => {
                let filename = config.content.stdin_filename.as_deref().unwrap_or("download");
                let attachment = format!("Content-Disposition: attachment; filename=\"{}\"\r\n", filename);
                let header = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\n{attachment}Server: {server_name}\r\nDate: {}\r\nConnection: close\r\nContent-Length: {}\r\n\r\n",
                    http_date(), data.len()
                );
                if stream.write_all(header.as_bytes()).is_err() {
                    return false;
                }
                stream.write_all(data).is_ok()
            }
            // if stdin is empty, then send it is text.
            None => {
                let text = config.content.text.as_deref().unwrap_or("No content");
                let header = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nServer: {server_name}\r\nDate: {}\r\nConnection: close\r\nContent-Length: {}\r\n\r\n",
                    http_date(), text.len()
                );
                if stream.write_all(header.as_bytes()).is_err() {
                    return false;
                }
                stream.write_all(text.as_bytes()).is_ok()
            }
        }
    }
}

fn send_file<W: Write>(mut file: File, stream: &mut W, quiet: bool) -> Result<(), std::io::Error> {
    let file_size = file.metadata().unwrap().size();
    if file_size < 1_048_576 {
        let served =  std::io::copy(&mut file, stream);
        return match served {
            Ok(_) => Ok(()),
            Err(e) => Err(e)
        }
    }

    let mut sent: u64 = 0;
    let mut buf = [0u8; 8192];
    let mut last_percent = 101u8;

    loop {
        let n = match file.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e)=> return Err(e)
        };

        match stream.write_all(&buf[..n]) {
            Ok(_) => {},
            Err(e) => return Err(e)
        }

        sent += n as u64;
        let percent = (sent * 100 / file_size) as u8;
        if percent != last_percent && !quiet {
            let mut sent_str = sent.to_string(); sent_str.truncate(sent.to_string().len()-6);
            let mut size_str = file_size.to_string(); size_str.truncate(size_str.len()-6);
            
            print!("\rDownloading: {}%  {} / {} MB", percent, sent/1000000, file_size/1000000);
            let _ = std::io::stdout().flush();
            last_percent = percent;
        }
    }

    Ok(())
}

// Tests __________________________________________
#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ContentConfig, SecurityConfig, ServerConfig};

    #[test]
    fn parse_valid_request_line() {
        let raw = "GET /secret HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let result = parse_request_line(raw).unwrap();
        assert_eq!(result, ("GET".into(), "/secret".into(), "HTTP/1.1".into()));
    }

    #[test]
    fn parse_post_request() {
        let raw = "POST /upload HTTP/1.0\r\n\r\n";
        let result = parse_request_line(raw).unwrap();
        assert_eq!(result, ("POST".into(), "/upload".into(), "HTTP/1.0".into()));
    }

    #[test]
    fn parse_missing_version() {
        let raw = "GET /secret\r\n\r\n";
        assert!(parse_request_line(raw).is_none());
    }

    #[test]
    fn parse_empty_string() {
        assert!(parse_request_line("").is_none());
    }

    #[test]
    fn send_404_contains_status_and_server() {
        let mut buf: Vec<u8> = Vec::new();
        send_404(&mut buf, "nginx");
        let response = String::from_utf8(buf).unwrap();
        assert!(response.contains("404 Not Found"));
        assert!(response.contains("Server: nginx"));
        assert!(response.contains("Connection: close"));
    }

    #[test]
    fn serve_text_content() {
        let config = Config {
            server: ServerConfig::default(),
            content: ContentConfig {
                text: Some("hello world".into()),
                from_file: None,
                stdin_data: None,
                stdin_filename: None,
            },
            security: SecurityConfig::default(),
        };
        let mut buf: Vec<u8> = Vec::new();
        let result = serve_content(&mut buf, &config, "test-server");
        assert!(result);
        let response = String::from_utf8(buf).unwrap();
        assert!(response.contains("200 OK"));
        assert!(response.contains("Server: test-server"));
        assert!(response.contains("Content-Length: 11"));
        assert!(response.ends_with("hello world"));
    }
}
