use std::io::{BufWriter, Read, Write};
use std::fs::File;
use std::net::TcpListener;
use std::sync::{Arc, Mutex, mpsc};

use crate::{
    config::Config,
    constants,
    utils,
    visitor::Visit,
};

mod http;
mod tls;
mod tor;

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
    log_file: &Mutex<Option<BufWriter<File>>>,
) -> HandleResult {
    // Request reading happens without holding the log lock,
    // so other threads can still log while we wait for data.
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
    let quiet = config.server.quiet.unwrap_or(false);

    if !tor_mode && !config.security.is_ip_allowed(&visit.ip) {
        { let mut lf = log_file.lock().unwrap(); utils::log_request(&visit, "blocked (IP not allowed)", &mut lf, quiet); }
        http::send_404(stream, server_name);
        let _ = stream.flush();
        return HandleResult::Continue;
    }

    if visit.endpoint != expected_url || !config.security.is_method_allowed(&visit.method) {
        { let mut lf = log_file.lock().unwrap(); utils::log_request(&visit, "", &mut lf, quiet); }
        http::send_404(stream, server_name);
        let _ = stream.flush();
        return HandleResult::Continue;
    }

    { let mut lf = log_file.lock().unwrap(); utils::log_request(&visit, "", &mut lf, quiet); }
    println!("seen!");
    let served = http::serve_content(stream, config, server_name);
    let _ = stream.flush();

    if served { HandleResult::Served } else { HandleResult::ServeError }
}

pub fn run(config: Config) -> Result<(), String> {
    if config.server.tor.unwrap_or(false) {
        return tor::run(config);
    }
    let port = config.server.port.unwrap_or_else(utils::random_port);
    let endpoint = config.server.endpoint.clone().unwrap_or_else(utils::random_endpoint);
    let expected_url = format!("/{}", endpoint);
    let server_name = config.server.server_name.as_deref().unwrap_or(constants::DEFAULT_SERVER_NAME).to_string();
    let insecure_http = config.server.insecure_http.unwrap_or(false);

    let tls_config = if !insecure_http { Some(tls::make_tls_config(&config)?) } else { None };
    let log_file = Arc::new(Mutex::new(utils::open_log_file(&config)?));
    let config = Arc::new(config);

    let bind_addr = if config.server.port_forwarded.unwrap_or(false) { "127.0.0.1" } else { "0.0.0.0" };
    let listener = TcpListener::bind(format!("{}:{}", bind_addr, port))
        .map_err(|e| format!("Failed to start server: {}", e))?;
    let local_addr = listener.local_addr()
        .map_err(|e| format!("Failed to get local address: {}", e))?;

    let hash = match &config.content.from_file {
        Some(path) => utils::sha256_file(path).unwrap_or_else(|| "hash error".to_string()),
        None => match &config.content.stdin_data {
            Some(data) => utils::sha256_bytes(data),
            None => {
                let text = config.content.text.as_deref().unwrap_or("No content");
                utils::sha256_text(text)
            }
        }
    };

    println!(
        "Server started\nport: {}\nendpoint: {}\nHash: {}\n{}",
        port,
        expected_url,
        hash,
        if !insecure_http { "https: true\n" } else { "https: FALSE" }
    );

    // Channel for worker threads to signal that content was served.
    let (tx, rx) = mpsc::sync_channel::<bool>(1);

    for stream in listener.incoming() {
        // Check if any thread already served content
        if let Ok(served) = rx.try_recv() {
            if let Ok(mut lf) = log_file.lock()
                && let Some(f) = lf.as_mut() { let _ = f.flush(); }
            return if served { Ok(()) } else { Err("Failed to serve content".to_string()) };
        }

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
        let _ = stream.set_read_timeout(Some(constants::REQUEST_TIMEOUT));

        let tx = tx.clone();
        let tls_config = tls_config.clone();
        let config = Arc::clone(&config);
        let expected_url = expected_url.clone();
        let server_name = server_name.clone();
        let log_file = Arc::clone(&log_file);

        std::thread::spawn(move || {
            let result = match &tls_config {
                Some(tls_cfg) => {
                    let conn = match rustls::ServerConnection::new(Arc::clone(tls_cfg)) {
                        Ok(c) => c,
                        Err(e) => { eprintln!("TLS connection error: {}", e); return; }
                    };
                    let mut tls_stream = rustls::StreamOwned::new(conn, stream);
                    handle_connection(&mut tls_stream, ip, &expected_url, &server_name, &config, &log_file)
                }
                None => {
                    let mut stream = stream;
                    handle_connection(&mut stream, ip, &expected_url, &server_name, &config, &log_file)
                }
            };

            match result {
                HandleResult::Continue => {}
                HandleResult::Served => {
                    let _ = tx.send(true);
                    // Wake up the main thread's listener.incoming() loop
                    let _ = std::net::TcpStream::connect(local_addr);
                }
                HandleResult::ServeError => {
                    let _ = tx.send(false);
                    let _ = std::net::TcpStream::connect(local_addr);
                }
            }
        });
    }

    Ok(())
}
