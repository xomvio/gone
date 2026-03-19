use clap::Parser;
use std::borrow::Cow;
use std::fs;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::path::Path;
use toml;

const DEFAULT_CONFIG: &str = include_str!("../default-config.toml");

// ── Config structs ────────────────────────────────────────────────────────────

#[derive(serde::Deserialize, Clone, Debug)]
pub struct Config {
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub content: ContentConfig,
    #[serde(default)]
    pub security: SecurityConfig,
}

#[derive(serde::Deserialize, Clone, Debug)]
pub struct ServerConfig {
    pub port: Option<u16>,
    pub content_type: Option<Cow<'static, str>>,
    pub server_name: Option<Cow<'static, str>>,
    pub endpoint: Option<String>,
    pub output: Option<String>,
}

#[derive(serde::Deserialize, Clone, Debug)]
pub struct ContentConfig {
    pub text: Option<Cow<'static, str>>,
    pub from_file: Option<String>,
}

#[derive(serde::Deserialize, Clone, Debug)]
pub struct SecurityConfig {
    /// Maximum number of visits per IP (0 = unlimited)
    pub max_visits: Option<u32>,
    pub allowed_methods: Option<Vec<String>>,
    pub blacklist: Option<Vec<String>>,
    pub whitelist: Option<Vec<String>>,
}

impl SecurityConfig {
    pub fn is_method_allowed(&self, method: &str) -> bool {
        match &self.allowed_methods {
            Some(methods) => methods.iter().any(|m| m.eq_ignore_ascii_case(method)),
            None => true,
        }
    }

    /// Whitelist takes priority: if non-empty, only listed IPs are allowed.
    /// Blacklisted IPs are always blocked (unless also whitelisted).
    pub fn is_ip_allowed(&self, ip: &str) -> bool {
        let whitelist_active = self.whitelist.as_ref().map(|wl| !wl.is_empty()).unwrap_or(false);

        if whitelist_active {
            return self.whitelist.as_ref()
                .map(|wl| wl.iter().any(|w| w == ip))
                .unwrap_or(false);
        }

        !self.blacklist.as_ref()
            .map(|bl| bl.iter().any(|b| b == ip))
            .unwrap_or(false)
    }
}

// ── Default impls ─────────────────────────────────────────────────────────────

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            content: ContentConfig::default(),
            security: SecurityConfig::default(),
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: None,
            content_type: Some(Cow::Borrowed("text/plain")),
            server_name: Some(Cow::Borrowed("nginx")),
            endpoint: None,
            output: None,
        }
    }
}

impl Default for ContentConfig {
    fn default() -> Self {
        Self {
            text: Some(Cow::Borrowed("This is a secret message that will be shown once.")),
            from_file: None,
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            max_visits: Some(3),
            allowed_methods: Some(vec!["GET".to_string()]),
            blacklist: Some(Vec::new()),
            whitelist: Some(Vec::new()),
        }
    }
}

// ── CLI args ──────────────────────────────────────────────────────────────────

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

// ── Loading ───────────────────────────────────────────────────────────────────

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
                eprintln!("Warning: Could not read {}: {}", args.config, e);
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
        config.server.endpoint = Some(endpoint);
    }
    if let Some(output) = args.output {
        config.server.output = Some(output);
    }
    if let Some(max_visits) = args.max_visits {
        config.security.max_visits = Some(max_visits);
    }

    validate(&config);
    config
}

// ── Validation ────────────────────────────────────────────────────────────────

fn validate(config: &Config) {
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

    if let Some(endpoint) = &config.server.endpoint {
        if !endpoint.starts_with('/') {
            eprintln!("Error: Endpoint must start with '/'");
            std::process::exit(1);
        }
    }

    validate_ip_list("blacklist", config.security.blacklist.as_deref().unwrap_or(&[]));
    validate_ip_list("whitelist", config.security.whitelist.as_deref().unwrap_or(&[]));
}

fn validate_ip_list(list_name: &str, ips: &[String]) {
    for ip in ips {
        if ip.parse::<Ipv4Addr>().is_err() && ip.parse::<Ipv6Addr>().is_err() {
            eprintln!("Error: Invalid IP address in {}: '{}'", list_name, ip);
            std::process::exit(1);
        }
    }
}
