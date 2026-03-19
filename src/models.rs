use std::u32;
use std::borrow::Cow;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::path::Path;

use serde::Deserialize;
use thiserror::Error;

/// Main configuration structure that holds all server settings
#[derive(Deserialize, Clone, Debug)]
pub struct Config {
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub content: ContentConfig,
    #[serde(default)]
    pub security: SecurityConfig,
}

/// Server-related configuration
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Invalid port number: {0}")]
    InvalidPort(u16),
    #[error("Invalid endpoint path: {0}")]
    InvalidEndpoint(String),
    #[error("Invalid IP address in {0}: {1}")]
    InvalidIpAddress(String, String),
}

#[derive(Deserialize, Clone, Debug)]
pub struct ServerConfig {
    /// Port to listen on (1024-65535)
    pub port: Option<u16>,
    /// Content-Type header for responses
    pub content_type: Option<Cow<'static, str>>,
    /// Server header value
    pub server_name: Option<Cow<'static, str>>,
    /// Custom endpoint path (must start with /)
    pub endpoint: Option<String>,
    /// Path to output log file
    pub output: Option<String>,
}

impl ServerConfig {
    /// Validates the server configuration
    pub fn validate(&self) -> Result<(), ConfigError> {
        if let Some(port) = self.port {
            if port < 1024 {
                return Err(ConfigError::InvalidPort(port));
            }
        }

        if let Some(endpoint) = &self.endpoint {
            if !endpoint.starts_with('/') {
                return Err(ConfigError::InvalidEndpoint(endpoint.clone()));
            }
        }

        Ok(())
    }
}

/// Content serving configuration
#[derive(Deserialize, Clone, Debug)]
pub struct ContentConfig {
    /// Text content to serve directly
    pub text: Option<Cow<'static, str>>,
    /// Path to file to serve (alternative to text)
    pub from_file: Option<String>,
}

/// Security-related configuration
#[derive(Deserialize, Clone, Debug)]
pub struct SecurityConfig {
    /// Maximum number of visits per IP (0 = unlimited)
    pub max_visits: Option<u32>,
    /// Allowed HTTP methods
    pub allowed_methods: Option<Vec<String>>,
    /// List of blocked IPs (supports CIDR notation)
    pub blacklist: Option<Vec<String>>,
    /// List of allowed IPs (if not empty, only these are allowed)
    pub whitelist: Option<Vec<String>>,
}

impl SecurityConfig {
    /// Validates IP addresses in blacklist and whitelist
    pub fn validate_ips(&self) -> Result<(), ConfigError> {
        if let Some(blacklist) = &self.blacklist {
            for ip in blacklist {
                if ip.parse::<Ipv4Addr>().is_err() && ip.parse::<Ipv6Addr>().is_err() {
                    return Err(ConfigError::InvalidIpAddress("blacklist".into(), ip.clone()));
                }
            }
        }

        if let Some(whitelist) = &self.whitelist {
            for ip in whitelist {
                if ip.parse::<Ipv4Addr>().is_err() && ip.parse::<Ipv6Addr>().is_err() {
                    return Err(ConfigError::InvalidIpAddress("whitelist".into(), ip.clone()));
                }
            }
        }

        Ok(())
    }

    /// Returns false if the IP should be blocked, true if allowed.
    /// Whitelist takes priority: if non-empty, only listed IPs are allowed.
    /// Blacklisted IPs are always blocked (unless also whitelisted).
    pub fn is_ip_allowed(&self, ip: &str) -> bool {
        // check if whitelist is undefined or empty. if not, return true.
        let whitelist_active = self.whitelist.as_ref().map(|wl| !wl.is_empty()).unwrap_or(false);

        if whitelist_active {
            let whitelisted = self.whitelist.as_ref().map(|wl| wl.iter().any(|w| w == ip)).unwrap_or(false);
            return whitelisted;
        }

        let in_blacklist = self.blacklist.as_ref()
            .map(|bl| bl.iter().any(|b| b == ip))
            .unwrap_or(false);

        !in_blacklist
    }
}

impl SecurityConfig {
    /// Checks if a given HTTP method is allowed
    pub fn is_method_allowed(&self, method: &str) -> bool {
        match &self.allowed_methods {
            Some(methods) => methods.iter()
                .any(|m| m.eq_ignore_ascii_case(method)),
            None => true, // if no allowed_methods defined, allow all
        }
    }
}

// Implement Default for all config sections
impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: None,
            content_type: Some(Cow::Borrowed("text/html")),
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

impl Config {
    /// Validates the entire configuration
    pub fn validate(&self) -> Result<(), ConfigError> {
        self.server.validate()?;
        self.security.validate_ips()?;
        
        // Add any additional cross-field validation here
        
        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            content: ContentConfig::default(),
            security: SecurityConfig::default(),
        }
    }
}

#[derive(Debug)]
pub struct Visit {
    pub datetime: String,
    pub ip: String,
    pub endpoint: String,
    pub method: String,
    pub version: String,
}

pub struct Visitor {
    pub visits: Vec<Visit>,
    pub blocked: bool,
}

impl Visitor {
    pub fn check(&mut self, _config: &Config) -> bool {
        if self.blocked {
            return true;
        }

        let mut blocked = false;
        if self.visits.len() > _config.security.max_visits.unwrap_or(u32::MAX) as usize {
            blocked = true;
        }

        self.blocked = blocked;
        blocked
    }
}
