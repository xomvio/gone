use std::u32;

use serde::Deserialize;

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
#[derive(Deserialize, Clone, Debug)]
pub struct ServerConfig {
    /// Port to listen on (1024-65535)
    pub port: Option<String>,
    /// Content-Type header for responses
    pub content_type: Option<String>,
    /// Server header value
    pub server_name: Option<String>,
    /// Custom endpoint path (must start with /)
    pub endpoint: Option<String>,
    /// Path to output log file
    pub output: Option<String>,
}

/// Content serving configuration
#[derive(Deserialize, Clone, Debug)]
pub struct ContentConfig {
    /// Text content to serve directly
    pub text: Option<String>,
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

// Implement Default for all config sections
impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: None,
            content_type: Some("text/html".to_string()),
            server_name: Some("nginx".to_string()),
            endpoint: None,
            output: None,
        }
    }
}

impl Default for ContentConfig {
    fn default() -> Self {
        Self {
            text: Some("This is a secret message that will be shown once.".to_string()),
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

        let last_visit = match self.visits.last() {
            Some(visit) => visit,
            None => return false, // No visits yet, not blocked
        };
        
        if last_visit.method == "POST" {
            blocked = true;
        }

        self.blocked = blocked;
        blocked
    }
}
