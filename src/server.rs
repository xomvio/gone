use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tiny_http::{Header, Response, Server};
use std::collections::HashMap;
use chrono;
use std::str::FromStr;

use crate::{models::{Config, Visitor, Visit}, utils};

pub fn run_server(config: Config) -> ! {
    // one use flag
    let used = Arc::new(AtomicBool::new(false));
    
    let port = match &config.port {
        Some(port) => port,
        None => &utils::random_port()
    };
    
    let endpoint = match &config.endpoint {
        Some(endpoint) => endpoint,
        None => &utils::random_endpoint()
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
        
        match visitors.get_mut(&remote_addr) {
            Some(visitor) => {
                visitor.visits.push(Visit {
                    datetime: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                    ip: remote_addr.clone(),
                    endpoint: request.url().to_string(),
                    method: request.method().as_str().to_string(),
                    version: request.http_version().to_string()
                });
            },
            None => {
                let first_visit = Visit {
                    datetime: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                    ip: remote_addr.clone(),
                    endpoint: request.url().to_string(),
                    method: request.method().as_str().to_string(),
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
{color_yellow}DateTime:{color_reset}{}
{color_yellow}IP:{color_reset}{}
{color_yellow}Enpoint:{color_reset}{}
{color_yellow}Method:{color_reset}{}
{color_yellow}Version:{color_reset}{}
{}"#,
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        remote_addr,
        request.url(),
        request.method().as_str(),
        request.http_version(),
        if blocked {"blocked\r\n"} else {""});

        // if visitor blocked, respond with 404
        if blocked || request.url() != ("/".to_string() + &endpoint) {
            let server_name = config.server_name.as_deref().unwrap_or("nginx");
            let server_header = format!("Server: {}", server_name);
            let header = match Header::from_str(&server_header) {
                Ok(h) => h,
                Err(_) => {
                    eprintln!("Failed to create header: {}", server_header);
                    std::process::exit(1);
                }
            };
            
            let resp = Response::new(
                tiny_http::StatusCode(404),
                vec![header],
                "".as_bytes(),
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

            let msg = match &config.from_file {
                Some(file_path) => match std::fs::read_to_string(file_path) {
                    Ok(content) => content,
                    Err(e) => {
                        eprintln!("Failed to read file: {}", e);
                        "Error reading file".to_string()
                    }
                },
                None => config.text.as_deref().unwrap_or("nothing").to_string()
            };
            
            let content_type = match Header::from_str(&format!("Content-Type: {}", config.content_type.as_deref().unwrap_or("text/html"))) {
                Ok(header) => header,
                Err(_) => {
                    eprintln!("Failed to create content type header: {}", config.content_type.as_deref().unwrap_or("text/html"));
                    std::process::exit(1);
                }
            };
            
            let server_hdr = match Header::from_str(&format!("Server: {}", config.server_name.as_deref().unwrap_or("nginx"))) {
                Ok(header) => header,
                Err(_) => {
                    eprintln!("Failed to create server header: {}", config.server_name.as_deref().unwrap_or("nginx"));
                    std::process::exit(1);
                }
            };
            
            let resp = Response::new(
                tiny_http::StatusCode(200),
                vec![content_type, server_hdr],
                msg.as_bytes(),
                Some(msg.len()),
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
