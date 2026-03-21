use std::fs::File;
use std::io::Write;

use crate::config::Config;
use crate::utils::cow_str_to_str;

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
        "HTTP/1.1 404 Not Found\r\nServer: {server_name}\r\nContent-Length: {}\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(body);
}

/// Sends an HTTP 200 response with the configured content.
///
/// Uses `std::io::copy` for file content — no full-file buffering, binary-safe.
/// Returns true on success, false on any read/write error.
pub fn serve_content<W: Write>(stream: &mut W, config: &Config, server_name: &str) -> bool {
    let content_type = cow_str_to_str(&config.server.content_type, "text/plain");

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
                "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nServer: {server_name}\r\nContent-Length: {size}\r\n\r\n"
            );
            if stream.write_all(header.as_bytes()).is_err() {
                return false;
            }
            std::io::copy(&mut file, stream).is_ok()
        }
        None => {
            let text = cow_str_to_str(&config.content.text, "No content");
            let header = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nServer: {server_name}\r\nContent-Length: {}\r\n\r\n",
                text.len()
            );
            if stream.write_all(header.as_bytes()).is_err() {
                return false;
            }
            stream.write_all(text.as_bytes()).is_ok()
        }
    }
}
