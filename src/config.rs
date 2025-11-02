use clap::Parser;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Default configuration values
const DEFAULT_CONFIG: &str = r#"
# Server configuration
port = "8080"
content_type = "text/html"
server_name = "nginx"

# Request handling
max_visits = 10
allowed_methods = ["GET"]

# Response configuration
from_file = ""  # Path to file to serve
text = ""       # Direct text to serve (alternative to from_file)
endpoint = ""   # Custom endpoint path
output = ""     # Output file path (if any)

# Security
blacklist = []  # List of IPs to block
whitelist = []  # List of allowed IPs (if not empty, only these are allowed)
"#;

/// Command line arguments
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Port to listen on
    #[arg(short, long)]
    pub port: Option<String>,

    /// Content-Type header value
    #[arg(short, long)]
    pub content_type: Option<String>,

    /// Server header value
    #[arg(short = 'n', long, default_value_t = String::from("nginx"))]
    pub server_name: String,

    /// Path to file to serve (alternative to --text)
    #[arg(short = 'f', long, value_name = "FILE")]
    pub from_file: Option<String>,

    /// Text to serve directly (alternative to --from-file)
    #[arg(short = 't', long)]
    pub text: Option<String>,

    /// Custom endpoint path (e.g., "/secret")
    #[arg(short = 'e', long)]
    pub endpoint: Option<String>,

    /// Output file path (if any)
    #[arg(short = 'o', long)]
    pub output: Option<String>,

    /// Generate a default config file and exit
    #[arg(long)]
    pub generate_config: bool,
}

/// Load configuration from file and merge with command line arguments
pub fn load() -> crate::models::Config {
    let args = Args::parse();

    // Handle config generation if requested
    if args.generate_config {
        if let Err(e) = fs::write("config.toml", DEFAULT_CONFIG) {
            eprintln!("Failed to generate config file: {}", e);
            std::process::exit(1);
        }
        println!("Configuration file 'config.toml' created successfully.");
        std::process::exit(0);
    }

    // Try to load config from file, or use defaults if not found
    let config_content = if Path::new("config.toml").exists() {
        match fs::read_to_string("config.toml") {
            Ok(content) => content,
            Err(e) => {
                eprintln!("Warning: Could not read config.toml: {}", e);
                String::from(DEFAULT_CONFIG)
            }
        }
    } else {
        String::from(DEFAULT_CONFIG)
    };

    // Parse the config file
    let mut config: crate::models::Config = match toml::from_str(&config_content) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Error parsing config.toml: {}", e);
            std::process::exit(1);
        }
    };

    // Override with command line arguments
    if let Some(port) = args.port {
        config.port = Some(port);
    }
    if let Some(content_type) = args.content_type {
        config.content_type = Some(content_type);
    }
    if args.server_name != "nginx" {  // Only override if not default
        config.server_name = Some(args.server_name);
    }
    if args.from_file.is_some() {
        config.from_file = args.from_file;
    }
    if args.text.is_some() {
        config.text = args.text;
    }
    if args.endpoint.is_some() {
        config.endpoint = args.endpoint;
    }
    if args.output.is_some() {
        config.output = args.output;
    }

    // Validate the final configuration
    validate_config(&config);
    
    config
}

/// Validate the configuration and exit with an error message if invalid
fn validate_config(config: &crate::models::Config) {
    // Ensure either from_file or text is provided
    if config.from_file.is_none() && config.text.is_none() {
        eprintln!("Error: You must specify either --from-file or --text");
        std::process::exit(1);
    }

    // Validate port if specified
    if let Some(port) = &config.port {
        if let Err(e) = port.parse::<u16>() {
            eprintln!("Error: Invalid port number '{}': {}", port, e);
            std::process::exit(1);
        }
    }
}