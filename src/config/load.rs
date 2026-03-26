use std::{fs, io::Read, path::Path};
use clap::Parser;
use crate::config::{Config, DEFAULT_CONFIG, args::Args, validate};
use crate::constants;

pub fn load() -> Result<Config, String> {
    let args = Args::parse();

    if args.generate_config {
        fs::write("config.toml", DEFAULT_CONFIG)
            .map_err(|e| format!("Failed to generate config file: {}", e))?;
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

    let mut config: Config = toml::from_str(&config_content)
        .map_err(|e| format!("Error parsing config file: {}", e))?;

    // Override with command line arguments
    if let Some(port_str) = args.port {
        config.server.port = Some(
            port_str.parse::<u16>()
                .map_err(|_| format!("Invalid port number '{}'", port_str))?
        );
    }
    if let Some(content_type) = &args.content_type {
        config.server.content_type = Some(content_type.to_owned());
    }
    if let Some(server_name) = args.server_name {
        config.server.server_name = Some(server_name);
    }
    if let Some(from_file) = args.from_file {
        if from_file == "-" {
            // Stdin mode: read all data from stdin
            let stdin_filename = args.stdin_filename
                .ok_or("stdin mode requires --stdin-filename (e.g., --from-file - --stdin-filename file.pdf)")?;
            let mut data = Vec::new();
            std::io::stdin().read_to_end(&mut data)
                .map_err(|e| format!("Failed to read from stdin: {}", e))?;
            config.content.stdin_data = Some(data);
            config.content.stdin_filename = Some(stdin_filename);
            config.content.from_file = None;
        } else {
            config.content.from_file = Some(from_file);
        }
        config.content.text = None; // CLI --from-file overrides config text
    }
    if let Some(text) = args.text {
        config.content.text = Some(text);
        config.content.from_file = None; // CLI --text overrides config from-file
    }
    if let Some(endpoint) = args.endpoint {
        config.server.endpoint = Some(endpoint.trim_start_matches('/').to_string());
    }
    if let Some(output) = args.output {
        config.server.output = Some(output);
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
    if args.quiet {
        config.server.quiet = Some(true);
    }

    // Normalize endpoint: strip leading '/' if present
    // Server adds it later
    if let Some(endpoint) = &mut config.server.endpoint {
        *endpoint = endpoint.trim_start_matches('/').to_string();
    }

    // Auto-set content-type to text/plain when serving text and no explicit content-type was set
    if config.content.text.is_some() && config.content.from_file.is_none() {
        let is_default = config.server.content_type.as_deref() == Some(constants::DEFAULT_CONTENT_TYPE);
        if is_default {
            config.server.content_type = Some("text/plain".to_string());
        }
    }

    validate(&config)?;
    Ok(config)
}
