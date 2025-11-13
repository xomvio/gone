use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tiny_http::{Header, Response, Server};
use std::collections::HashMap;
use chrono;
use std::str::FromStr;
use std::borrow::Cow;

use crate::{
    models::{Config, Visitor, Visit},
    utils
};

// Helper function to get a string slice from Cow<str> or default
fn cow_str_to_str<'a>(cow: &'a Option<Cow<'static, str>>, default: &'static str) -> &'a str {
    cow.as_deref().unwrap_or(default)
}

/// Helper function to create a header with proper error handling
fn create_header(name: &str, value: &str) -> Header {
    let header_str = format!("{}: {}", name, value);
    match Header::from_str(&header_str) {
        Ok(header) => header,
        Err(_) => {
            eprintln!("Failed to create header: {}: {}", name, value);
            std::process::exit(1);
        }
    }
}

pub fn run_server(config: Config) -> ! {
    // one use flag
    let used = Arc::new(AtomicBool::new(false));
    
    let port = match config.server.port {
        Some(port) => port,
        None => utils::random_port()
    };
    
    let endpoint = match config.server.endpoint.clone() {
        Some(endpoint) => endpoint,
        None => utils::random_endpoint()
    };

    // tiny_http server
    let server = match Server::http(format!("0.0.0.0:{}", port)) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to start server: {}", e);
            std::process::exit(1);
        }
    };

    // colors for terminal output
    let color_green = "\x1b[92m";
    let color_reset = "\x1b[0m";
    let color_yellow = "\x1b[93m";

    // key is ip address
    let mut visitors: HashMap<String, Visitor> = HashMap::new();

    println!("Server started \r\nport: {} \r\nendpoint: {}\r\n", port, endpoint);

    for request in server.incoming_requests() {
        let remote_addr = match request.remote_addr() {
            Some(addr) => addr.to_string(),
            None => {
                eprintln!("Could not get remote address");
                continue;
            }
        };

        let method = request.method().as_str();
        
        match visitors.get_mut(&remote_addr) {
            Some(visitor) => {
                visitor.visits.push(Visit {
                    datetime: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                    ip: remote_addr.clone(),
                    endpoint: request.url().to_string(),
                    method: method.to_string(),
                    version: request.http_version().to_string()
                });
            },
            None => {
                let first_visit = Visit {
                    datetime: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                    ip: remote_addr.clone(),
                    endpoint: request.url().to_string(),
                    method: method.to_string(),
                    version: request.http_version().to_string()
                };
                visitors.insert(remote_addr.clone(), Visitor {
                    visits: vec![first_visit],
                    blocked: false
                });
            }
        }

        let blocked = if let Some(visitor) = visitors.get_mut(&remote_addr) {
            visitor.check(&config)
        } else {
            false
        };

        println!(r#"{color_green}Request{color_reset}
{color_yellow}DateTime:{color_reset} {}
{color_yellow}IP:{color_reset} {}
{color_yellow}Endpoint:{color_reset} {}
{color_yellow}Method:{color_reset} {}
{color_yellow}Version:{color_reset} {}
{}"#, 
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
            remote_addr, 
            request.url(), 
            method,
            request.http_version(), 
            if blocked { "blocked\r\n" } else { "" }
        );

        // if visitor blocked, respond with 404
        if blocked 
        || request.url() != format!("/{}", endpoint)
        || !config.security.is_method_allowed(method)
        {
            let server_name = cow_str_to_str(&config.server.server_name, "sdHTTPp");
            let resp = Response::new(
                tiny_http::StatusCode(404),
                vec![create_header("Server", server_name)],
                "404 Not Found".as_bytes(),
                None,
                None
            );
            if let Err(e) = request.respond(resp) {
                eprintln!("Failed to send response: {}", e);
            }
            continue;
        }

        if !used.swap(true, Ordering::SeqCst) {
            // First and only access - show the message
            println!("seen!");

            let (content, content_type) = match &config.content.from_file {
                Some(file_path) => match std::fs::read_to_string(file_path) {
                    Ok(content) => (content, cow_str_to_str(&config.server.content_type, "text/plain")),
                    Err(e) => {
                        eprintln!("Failed to read file: {}", e);
                        ("Error reading file".to_string(), "text/plain")
                    }
                },
                None => (
                    cow_str_to_str(&config.content.text, "No content").to_string(),
                    cow_str_to_str(&config.server.content_type, "text/plain")
                )
            };
            
            let server_name = cow_str_to_str(&config.server.server_name, "sdHTTPp");
            let resp = Response::new(
                tiny_http::StatusCode(200),
                vec![
                    create_header("Content-Type", content_type),
                    create_header("Server", server_name),
                ],
                content.as_bytes(),
                Some(content.len()),
                None
            );
            
            if let Err(e) = request.respond(resp) {
                eprintln!("Failed to send response: {}", e);
            }
            
            std::process::exit(0);
        } else {
            // For all other accesses, return error
            if let Err(e) = request.respond(Response::from_string("nothing")) {
                eprintln!("Failed to send response: {}", e);
            }
        }
    }
    
    // This should never be reached
    std::process::exit(0);
}
