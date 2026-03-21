use std::io::{BufWriter, Read, Write};
use std::fs::File;
use std::net::TcpListener;
use std::sync::Arc;
use std::time::Duration;

use crate::{
    config::Config,
    utils,
    visitor::Visit,
};

mod http;
mod tls;
#[cfg(feature = "tor")]
mod tor;

/// Maximum time to wait for a complete HTTP request (slowloris protection).
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

pub(crate) enum HandleResult {
    Continue,
    Served,
    ServeError,
}

pub(crate) fn handle_connection<S: Read + Write>(
    stream: &mut S,
    ip: String,
    expected_url: &str,
    server_name: &str,
    config: &Config,
    log_file: &mut Option<BufWriter<File>>,
) -> HandleResult {
    let raw = match utils::read_request(stream) {
        Some(r) => r,
        None => return HandleResult::Continue,
    };
    let (method, url, version) = match http::parse_request_line(&raw) {
        Some(t) => t,
        None => return HandleResult::Continue,
    };

    let visit = Visit {
        datetime: utils::now_str(),
        ip,
        endpoint: url,
        method,
        version,
    };

    let tor_mode = config.server.tor.unwrap_or(false);

    if !tor_mode && !config.security.is_ip_allowed(&visit.ip) {
        utils::log_request(&visit, "blocked (IP not allowed)", log_file);
        http::send_404(stream, server_name);
        let _ = stream.flush();
        return HandleResult::Continue;
    }

    if visit.endpoint != expected_url || !config.security.is_method_allowed(&visit.method) {
        utils::log_request(&visit, "", log_file);
        http::send_404(stream, server_name);
        let _ = stream.flush();
        return HandleResult::Continue;
    }

    utils::log_request(&visit, "", log_file);
    println!("seen!");
    let served = http::serve_content(stream, config, server_name);
    let _ = stream.flush();
    if served { HandleResult::Served } else { HandleResult::ServeError }
}

pub fn run(config: Config) -> Result<(), String> {
    #[cfg(feature = "tor")]
    if config.server.tor.unwrap_or(false) {
        return tor::run(config);
    }
    let port = config.server.port.unwrap_or_else(utils::random_port);
    let endpoint = config.server.endpoint.clone().unwrap_or_else(utils::random_endpoint);
    let expected_url = format!("/{}", endpoint);
    let server_name = config.server.server_name.as_deref().unwrap_or("nginx").to_string();
    let insecure_http = config.server.insecure_http.unwrap_or(false);

    let tls_config = if !insecure_http { Some(tls::make_tls_config(&config)?) } else { None };
    let mut log_file = utils::open_log_file(&config)?;

    let bind_addr = if config.server.port_forwarded.unwrap_or(false) { "127.0.0.1" } else { "0.0.0.0" };
    let listener = TcpListener::bind(format!("{}:{}", bind_addr, port))
        .map_err(|e| format!("Failed to start server: {}", e))?;

    println!(
        "Server started\nport: {}\nendpoint: {}\n{}",
        port,
        expected_url,
        if !insecure_http { "https: true\n" } else { "https: FALSE" }
    );

    for stream in listener.incoming() {
        let stream = match stream {
            Ok(s) => s,
            Err(e) => { eprintln!("TCP accept error: {}", e); continue; }
        };

        let ip = match stream.peer_addr() {
            Ok(addr) => addr.ip().to_string(),
            Err(_) => { eprintln!("Could not get remote address"); continue; }
        };

        // Set read timeout to protect against slowloris attacks.
        // This applies to both plain TCP and TLS (which wraps the same TcpStream).
        let _ = stream.set_read_timeout(Some(REQUEST_TIMEOUT));

        let result = match &tls_config {
            Some(tls_cfg) => {
                let conn = match rustls::ServerConnection::new(Arc::clone(tls_cfg)) {
                    Ok(c) => c,
                    Err(e) => { eprintln!("TLS connection error: {}", e); continue; }
                };
                let mut tls_stream = rustls::StreamOwned::new(conn, stream);
                handle_connection(&mut tls_stream, ip, &expected_url, &server_name, &config, &mut log_file)
            }
            None => {
                let mut stream = stream;
                handle_connection(&mut stream, ip, &expected_url, &server_name, &config, &mut log_file)
            }
        };

        match result {
            HandleResult::Continue => continue,
            HandleResult::Served => {
                if let Some(f) = &mut log_file { let _ = f.flush(); }
                return Ok(());
            }
            HandleResult::ServeError => {
                if let Some(f) = &mut log_file { let _ = f.flush(); }
                return Err("Failed to serve content".to_string());
            }
        }
    }

    Ok(())
}
