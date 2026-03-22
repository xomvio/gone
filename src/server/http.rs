use std::fs::File;
use std::io::Write;

use crate::{config::Config, constants};

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
        "HTTP/1.1 404 Not Found\r\nServer: {server_name}\r\nConnection: close\r\nContent-Length: {}\r\n\r\n",
        body.len()
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
        Some(path) => {
            let mut file = match File::open(path) {
                Ok(f) => f,
                Err(e) => {
                    eprintln!("Failed to open file '{}': {}", path, e);
                    return false;
                }
            };
            let size = match file.metadata() {
                Ok(m) => m.len(),
                Err(e) => {
                    eprintln!("Failed to stat file '{}': {}", path, e);
                    return false;
                }
            };
            let header = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nServer: {server_name}\r\nConnection: close\r\nContent-Length: {size}\r\n\r\n"
            );
            if stream.write_all(header.as_bytes()).is_err() {
                return false;
            }
            std::io::copy(&mut file, stream).is_ok()
        }
        None => {
            let text = config.content.text.as_deref().unwrap_or("No content");
            let header = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nServer: {server_name}\r\nConnection: close\r\nContent-Length: {}\r\n\r\n",
                text.len()
            );
            if stream.write_all(header.as_bytes()).is_err() {
                return false;
            }
            stream.write_all(text.as_bytes()).is_ok()
        }
    }
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
