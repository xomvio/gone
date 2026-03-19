use std::io::{BufWriter, Write};
use std::fs::{File, OpenOptions};
use tiny_http::{Header, Response, Server};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::str::FromStr;
use chrono;

use crate::{
    config::Config,
    visitor::{Visitor, Visit},
    utils,
};

const COLOR_GREEN: &str = "\x1b[92m";
const COLOR_RESET: &str = "\x1b[0m";
const COLOR_YELLOW: &str = "\x1b[93m";

fn cow_str_to_str<'a>(cow: &'a Option<std::borrow::Cow<'static, str>>, default: &'static str) -> &'a str {
    cow.as_deref().unwrap_or(default)
}

fn create_header(name: &str, value: &str) -> Header {
    let header_str = format!("{}: {}", name, value);
    match Header::from_str(&header_str) {
        Ok(h) => h,
        Err(_) => {
            eprintln!("Failed to create header: {}: {}", name, value);
            std::process::exit(1);
        }
    }
}

fn now_str() -> String {
    chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

fn extract_ip(request: &tiny_http::Request) -> Option<String> {
    request.remote_addr().map(|addr| {
        let s = addr.to_string();
        s.parse::<SocketAddr>()
            .map(|a| a.ip().to_string())
            .unwrap_or(s)
    })
}

fn log_request(
    now: &str, ip: &str, url: &str, method: &str, version: &str, status: &str,
    log_file: &mut Option<BufWriter<File>>,
) {
    let plain = format!(
        "Request\nDateTime: {now}\nIP: {ip}\nEndpoint: {url}\nMethod: {method}\nVersion: {version}{}",
        if status.is_empty() { String::new() } else { format!("\n{status}") }
    );

    println!(
        "{COLOR_GREEN}Request{COLOR_RESET}\n\
         {COLOR_YELLOW}DateTime:{COLOR_RESET} {now}\n\
         {COLOR_YELLOW}IP:{COLOR_RESET} {ip}\n\
         {COLOR_YELLOW}Endpoint:{COLOR_RESET} {url}\n\
         {COLOR_YELLOW}Method:{COLOR_RESET} {method}\n\
         {COLOR_YELLOW}Version:{COLOR_RESET} {version}{}",
        if status.is_empty() { String::new() } else { format!("\n{status}") }
    );

    if let Some(f) = log_file {
        let _ = writeln!(f, "{}", plain);
    }
}

fn send_404(request: tiny_http::Request, server_name: &str) {
    let resp = Response::new(
        tiny_http::StatusCode(404),
        vec![create_header("Server", server_name)],
        "404 Not Found".as_bytes(),
        None,
        None,
    );
    if let Err(e) = request.respond(resp) {
        eprintln!("Failed to send response: {}", e);
    }
}

/// Returns true on success, false if content could not be loaded.
fn serve_content(request: tiny_http::Request, config: &Config, server_name: &str) -> bool {
    let (content, content_type) = match &config.content.from_file {
        Some(path) => match std::fs::read_to_string(path) {
            Ok(c) => (c, cow_str_to_str(&config.server.content_type, "text/plain")),
            Err(e) => {
                eprintln!("Failed to read file '{}': {}", path, e);
                return false;
            }
        },
        None => (
            cow_str_to_str(&config.content.text, "No content").to_string(),
            cow_str_to_str(&config.server.content_type, "text/plain"),
        ),
    };

    let resp = Response::new(
        tiny_http::StatusCode(200),
        vec![
            create_header("Content-Type", content_type),
            create_header("Server", server_name),
        ],
        content.as_bytes(),
        Some(content.len()),
        None,
    );
    if let Err(e) = request.respond(resp) {
        eprintln!("Failed to send response: {}", e);
    }
    true
}

pub fn run_server(config: Config) -> ! {
    let port = config.server.port.unwrap_or_else(utils::random_port);
    let endpoint = config.server.endpoint.clone().unwrap_or_else(utils::random_endpoint);
    let server_name = cow_str_to_str(&config.server.server_name, "sdHTTPp").to_string();

    let mut log_file: Option<BufWriter<File>> = config.server.output.as_ref().map(|path| {
        OpenOptions::new().create(true).append(true).open(path)
            .unwrap_or_else(|e| {
                eprintln!("Failed to open log file '{}': {}", path, e);
                std::process::exit(1);
            })
    }).map(BufWriter::new);

    let server = Server::http(format!("0.0.0.0:{}", port)).unwrap_or_else(|e| {
        eprintln!("Failed to start server: {}", e);
        std::process::exit(1);
    });

    let mut visitors: HashMap<String, Visitor> = HashMap::new();

    println!("Server started\nport: {}\nendpoint: {}\n", port, endpoint);

    for request in server.incoming_requests() {
        let Some(remote_ip) = extract_ip(&request) else {
            eprintln!("Could not get remote address");
            continue;
        };

        let method  = request.method().as_str().to_string();
        let version = request.http_version().to_string();
        let url     = request.url().to_string();
        let now     = now_str();

        if !config.security.is_ip_allowed(&remote_ip) {
            log_request(&now, &remote_ip, &url, &method, &version, "blocked (IP not allowed)", &mut log_file);
            send_404(request, &server_name);
            continue;
        }

        let visitor = visitors.entry(remote_ip.clone()).or_insert_with(Visitor::new);
        visitor.visits.push(Visit {
            datetime: now.clone(), ip: remote_ip.clone(),
            endpoint: url.clone(), method: method.clone(), version: version.clone(),
        });
        let blocked = visitor.check(&config);

        log_request(&now, &remote_ip, &url, &method, &version, if blocked { "blocked" } else { "" }, &mut log_file);

        if blocked || url != format!("/{}", endpoint) || !config.security.is_method_allowed(&method) {
            send_404(request, &server_name);
            continue;
        }

        println!("seen!");
        if serve_content(request, &config, &server_name) {
            std::process::exit(0);
        } else {
            std::process::exit(1);
        }
    }

    std::process::exit(0);
}
