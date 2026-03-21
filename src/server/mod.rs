use std::collections::HashMap;
use std::io::{BufWriter, Read, Write};
use std::fs::File;
use std::net::TcpListener;
use std::sync::Arc;

use crate::{
    config::Config,
    http,
    tls,
    utils,
    visitor::{Visit, Visitor},
};

enum HandleResult {
    Continue,
    Served,
    ServeError,
}

fn handle_connection<S: Read + Write>(
    stream: &mut S,
    ip: String,
    expected_url: &str,
    server_name: &str,
    config: &Config,
    visitors: &mut HashMap<String, Visitor>,
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

    if !config.security.is_ip_allowed(&visit.ip) {
        utils::log_request(&visit, "blocked (IP not allowed)", log_file);
        http::send_404(stream, server_name);
        return HandleResult::Continue;
    }

    let visitor = visitors.entry(visit.ip.clone()).or_insert_with(Visitor::new);
    visitor.visits.push(visit.clone());
    let blocked = visitor.check(config);

    utils::log_request(&visit, if blocked { "blocked" } else { "" }, log_file);

    if blocked || visit.endpoint != expected_url || !config.security.is_method_allowed(&visit.method) {
        http::send_404(stream, server_name);
        return HandleResult::Continue;
    }

    println!("seen!");
    if http::serve_content(stream, config, server_name) {
        HandleResult::Served
    } else {
        HandleResult::ServeError
    }
}

pub fn run(config: Config) -> ! {
    let port = config.server.port.unwrap_or_else(utils::random_port);
    let endpoint = config.server.endpoint.clone().unwrap_or_else(utils::random_endpoint);
    let expected_url = format!("/{}", endpoint);
    let server_name = utils::cow_str_to_str(&config.server.server_name, "nginx").to_string();
    let https = config.server.https.unwrap_or(false);

    let tls_config = if https { Some(tls::make_tls_config(&config)) } else { None };
    let mut log_file = utils::open_log_file(&config);
    let mut visitors: HashMap<String, Visitor> = HashMap::new();

    let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).unwrap_or_else(|e| {
        eprintln!("Failed to start server: {}", e);
        std::process::exit(1);
    });

    println!(
        "Server started\nport: {}\nendpoint: {}\n{}",
        port,
        expected_url,
        if https { "https: true\n" } else { "" }
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

        let result = match &tls_config {
            Some(tls_cfg) => {
                let conn = match rustls::ServerConnection::new(Arc::clone(tls_cfg)) {
                    Ok(c) => c,
                    Err(e) => { eprintln!("TLS connection error: {}", e); continue; }
                };
                let mut tls_stream = rustls::StreamOwned::new(conn, stream);
                handle_connection(&mut tls_stream, ip, &expected_url, &server_name, &config, &mut visitors, &mut log_file)
            }
            None => {
                let mut stream = stream;
                handle_connection(&mut stream, ip, &expected_url, &server_name, &config, &mut visitors, &mut log_file)
            }
        };

        match result {
            HandleResult::Continue => continue,
            HandleResult::Served | HandleResult::ServeError => {
                if let Some(f) = &mut log_file { let _ = f.flush(); }
                std::process::exit(if matches!(result, HandleResult::Served) { 0 } else { 1 });
            }
        }
    }

    std::process::exit(0);
}
