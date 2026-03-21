use crate::config::{ContentConfig, SecurityConfig, ServerConfig};

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: None,
            content_type: Some("text/plain".to_string()),
            server_name: Some("nginx".to_string()),
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
            text: Some("This is a secret message that will be shown once.".to_string()),
            from_file: None,
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            allowed_methods: Some(vec!["GET".to_string()]),
            blacklist: Some(Vec::new()),
            whitelist: Some(Vec::new()),
        }
    }
}
