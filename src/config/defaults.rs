use std::borrow::Cow;

use crate::config::{Config, ContentConfig, SecurityConfig, ServerConfig};


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
            insecure_http: None,
            tor: None,
            port_forwarded: None,
            cert_path: None,
            key_path: None,
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
