use clap::Parser;
use std::fs;
use std::path::Path;
use toml;

/// Default configuration values
const DEFAULT_CONFIG: &str = include_str!("../default-config.toml");

/// Command line arguments
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Port to listen on (1024-65535)
    #[arg(short, long)]
    pub port: Option<String>,

    /// Content-Type header for the response (e.g., "text/plain", "text/html")
    #[arg(long, value_name = "TYPE")]
    pub content_type: Option<String>,

    /// Server header value (default: "nginx")
    #[arg(long, value_name = "NAME")]
    pub server_name: Option<String>,

    /// Path to file to serve (alternative to --text)
    #[arg(long, value_name = "FILE")]
    pub from_file: Option<String>,

    /// Text to serve directly (alternative to --from-file)
    #[arg(long, value_name = "TEXT")]
    pub text: Option<String>,

    /// Custom endpoint path (must start with /)
    #[arg(long, value_name = "PATH")]
    pub endpoint: Option<String>,

    /// Path to output log file (default: stdout)
    #[arg(long, value_name = "FILE")]
    pub output: Option<String>,

    /// Maximum number of visits per IP (0 for unlimited)
    #[arg(long, value_name = "COUNT")]
    pub max_visits: Option<u32>,

    /// Generate a default config file and exit
    #[arg(long)]
    pub generate_config: bool,

    /// Path to config file (default: config.toml in current directory)
    #[arg(short, long, default_value = "config.toml")]
    pub config: String,
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
    let config_content = if Path::new(&args.config).exists() {
        match fs::read_to_string(&args.config) {
            Ok(content) => content,
            Err(e) => {
                eprintln!("Warning: Could not read {}: {}", args.config, e);
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
            eprintln!("Error parsing config file: {}", e);
            std::process::exit(1);
        }
    };

    // Override with command line arguments
    if let Some(port) = args.port {
        config.server.port = Some(port);
    }
    if let Some(content_type) = args.content_type {
        config.server.content_type = Some(content_type);
    }
    if let Some(server_name) = args.server_name {
        config.server.server_name = Some(server_name);
    }
    if let Some(from_file) = args.from_file {
        config.content.from_file = Some(from_file);
    }
    if let Some(text) = args.text {
        config.content.text = Some(text);
    }
    if let Some(endpoint) = args.endpoint {
        config.server.endpoint = Some(endpoint);
    }
    if let Some(output) = args.output {
        config.server.output = Some(output);
    }
    if let Some(max_visits) = args.max_visits {
        config.security.max_visits = Some(max_visits);
    }

    // Validate the configuration
    validate_config(&config);
    
    config
}

/// Validate the configuration and exit with an error message if invalid
fn validate_config(config: &crate::models::Config) {
    // Ensure either from_file or text is provided
    if config.content.from_file.is_none() && config.content.text.is_none() {
        eprintln!("Error: You must specify either --from-file or --text");
        std::process::exit(1);
    }

    // Validate port if specified
    if let Some(port) = &config.server.port {
        if port.parse::<u16>().is_err() {
            eprintln!("Error: Invalid port number '{}'", port);
            std::process::exit(1);
        }
    }
}