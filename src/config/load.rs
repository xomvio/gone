use std::{borrow::Cow, fs, path::Path};
use clap::Parser;
use toml;

use crate::config::{Config, DEFAULT_CONFIG, args::Args, validate};

pub fn load() -> Config {
    let args = Args::parse();

    if args.generate_config {
        if let Err(e) = fs::write("config.toml", DEFAULT_CONFIG) {
            eprintln!("Failed to generate config file: {}", e);
            std::process::exit(1);
        }
        println!("Configuration file 'config.toml' created successfully.");
        std::process::exit(0);
    }

    let config_content = if Path::new(&args.config).exists() {
        match fs::read_to_string(&args.config) {
            Ok(content) => content,
            Err(e) => {
                eprintln!("Warning: Could not read {}: {}\n Using default settings", args.config, e);
                String::from(DEFAULT_CONFIG)
            }
        }
    } else {
        String::from(DEFAULT_CONFIG)
    };

    let mut config: Config = match toml::from_str(&config_content) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Error parsing config file: {}", e);
            std::process::exit(1);
        }
    };

    // Override with command line arguments
    if let Some(port_str) = args.port {
        config.server.port = Some(match port_str.parse::<u16>() {
            Ok(p) => p,
            Err(_) => {
                eprintln!("Error: Invalid port number '{}'", port_str);
                std::process::exit(1);
            }
        });
    }
    if let Some(content_type) = args.content_type {
        config.server.content_type = Some(Cow::Owned(content_type));
    }
    if let Some(server_name) = args.server_name {
        config.server.server_name = Some(Cow::Owned(server_name));
    }
    if let Some(from_file) = args.from_file {
        config.content.from_file = Some(from_file);
    }
    if let Some(text) = args.text {
        config.content.text = Some(Cow::Owned(text));
    }
    if let Some(endpoint) = args.endpoint {
        config.server.endpoint = Some(endpoint.trim_start_matches('/').to_string());
    }
    if let Some(output) = args.output {
        config.server.output = Some(output);
    }
    if let Some(max_visits) = args.max_visits {
        config.security.max_visits = Some(max_visits);
    }
    if let Some(allowed_methods) = args.allowed_methods {
        config.security.allowed_methods = Some(allowed_methods);
    }
    if let Some(blacklist) = args.blacklist {
        config.security.blacklist = Some(blacklist);
    }
    if let Some(whitelist) = args.whitelist {
        config.security.whitelist = Some(whitelist);
    }
    if args.insecure_http {
        config.server.insecure_http = Some(true);
    }
    if args.tor {
        config.server.tor = Some(true);
    }
    if args.port_forwarded {
        config.server.port_forwarded = Some(true);
    }
    if let Some(cert_path) = args.cert_path {
        config.server.cert_path = Some(cert_path);
    }
    if let Some(key_path) = args.key_path {
        config.server.key_path = Some(key_path);
    }

    // Normalize endpoint: strip leading '/' if present
    // Server adds it later
    if let Some(endpoint) = &mut config.server.endpoint {
        *endpoint = endpoint.trim_start_matches('/').to_string();
    }

    validate(&config);
    config
}