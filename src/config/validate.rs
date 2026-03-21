use std::{net::{Ipv4Addr, Ipv6Addr}, path::Path};

use crate::config::Config;


pub fn validate(config: &Config) -> Result<(), String> {
    if config.content.from_file.is_none() && config.content.text.is_none() {
        return Err("You must specify either --from-file or --text".to_string());
    }

    if let Some(port) = config.server.port {
        if port < 1024 {
            return Err(format!("Invalid port number '{}' (must be 1024-65535)", port));
        }
    }

    validate_ip_list("blacklist", config.security.blacklist.as_deref().unwrap_or(&[]))?;
    validate_ip_list("whitelist", config.security.whitelist.as_deref().unwrap_or(&[]))?;

    if let Some(path) = &config.content.from_file {
        if path.contains("..") {
            return Err("--from-file path must not contain '..' for security reasons.".to_string());
        }
        if !Path::new(path).exists() {
            return Err(format!("File not found: '{}'", path));
        }
    }

    if config.server.tor.unwrap_or(false) && config.server.port_forwarded.unwrap_or(false) {
        return Err("--tor and --port-forwarded cannot be used together.".to_string());
    }

    let has_cert = config.server.cert_path.is_some();
    let has_key  = config.server.key_path.is_some();
    if has_cert != has_key {
        return Err("--cert-path and --key-path must be provided together.".to_string());
    }
    for (name, path_opt) in [("cert-path", &config.server.cert_path), ("key-path", &config.server.key_path)] {
        if let Some(path) = path_opt {
            if path.contains("..") {
                return Err(format!("--{name} must not contain '..'"));
            }
            if !Path::new(path).exists() {
                return Err(format!("File not found for --{name}: '{path}'"));
            }
        }
    }

    Ok(())
}

fn validate_ip_list(list_name: &str, ips: &[String]) -> Result<(), String> {
    for ip in ips {
        if ip.parse::<Ipv4Addr>().is_err() && ip.parse::<Ipv6Addr>().is_err() {
            return Err(format!("Invalid IP address in {}: '{}'", list_name, ip));
        }
    }
    Ok(())
}
