use std::{net::{Ipv4Addr, Ipv6Addr}, path::Path};

use crate::config::Config;


pub fn validate(config: &Config) {
    if config.content.from_file.is_none() && config.content.text.is_none() {
        eprintln!("Error: You must specify either --from-file or --text");
        std::process::exit(1);
    }

    if let Some(port) = config.server.port {
        if port < 1024 {
            eprintln!("Error: Invalid port number '{}' (must be 1024-65535)", port);
            std::process::exit(1);
        }
    }


    validate_ip_list("blacklist", config.security.blacklist.as_deref().unwrap_or(&[]));
    validate_ip_list("whitelist", config.security.whitelist.as_deref().unwrap_or(&[]));

    if let Some(path) = &config.content.from_file {
        if path.contains("..") {
            eprintln!("Error: --from-file path must not contain '..' for security reasons.");
            std::process::exit(1);
        }
        if !Path::new(path).exists() {
            eprintln!("Error: File not found: '{}'", path);
            std::process::exit(1);
        }
    }

    let has_cert = config.server.cert_path.is_some();
    let has_key  = config.server.key_path.is_some();
    if has_cert != has_key {
        eprintln!("Error: --cert-path and --key-path must be provided together.");
        std::process::exit(1);
    }
    for (name, path_opt) in [("cert-path", &config.server.cert_path), ("key-path", &config.server.key_path)] {
        if let Some(path) = path_opt {
            if path.contains("..") {
                eprintln!("Error: --{name} must not contain '..'");
                std::process::exit(1);
            }
            if !Path::new(path).exists() {
                eprintln!("Error: File not found for --{name}: '{path}'");
                std::process::exit(1);
            }
        }
    }
}

fn validate_ip_list(list_name: &str, ips: &[String]) {
    for ip in ips {
        if ip.parse::<Ipv4Addr>().is_err() && ip.parse::<Ipv6Addr>().is_err() {
            eprintln!("Error: Invalid IP address in {}: '{}'", list_name, ip);
            std::process::exit(1);
        }
    }
}