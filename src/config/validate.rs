use std::{net::{Ipv4Addr, Ipv6Addr}, path::Path};

use crate::config::Config;
use crate::constants;


pub fn validate(config: &Config) -> Result<(), String> {
    if config.content.from_file.is_none() && config.content.text.is_none() && config.content.stdin_data.is_none() {
        return Err("You must specify either --from-file or --text".to_string());
    }

    if let Some(port) = config.server.port && port < constants::MIN_PORT {
        return Err(format!("Invalid port number '{}' (must be {}-65535)", port, constants::MIN_PORT));
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
    if (has_cert || has_key) && config.server.insecure_http.unwrap_or(false) {
        return Err("--cert-path/--key-path and --insecure-http cannot be used together.".to_string());
    }
    if (has_cert || has_key) && config.server.tor.unwrap_or(false) {
        return Err("--cert-path/--key-path are not used in Tor mode (Tor handles encryption).".to_string());
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

// Tests __________________________________________
#[cfg(test)]
mod tests {
    use crate::config::{Config, ContentConfig, SecurityConfig, ServerConfig};
    use super::validate;

    fn config_with_text(text: &str) -> Config {
        Config {
            content: ContentConfig {
                text: Some(text.into()),
                from_file: None,
                stdin_data: None,
                stdin_filename: None,
            },
            server: ServerConfig::default(),
            security: SecurityConfig::default(),
        }
    }

    #[test]
    fn valid_text_config() {
        assert!(validate(&config_with_text("hello")).is_ok());
    }

    #[test]
    fn no_content_fails() {
        let config = Config {
            content: ContentConfig { text: None, from_file: None, stdin_data: None, stdin_filename: None },
            server: ServerConfig::default(),
            security: SecurityConfig::default(),
        };
        assert!(validate(&config).unwrap_err().contains("--from-file or --text"));
    }

    #[test]
    fn port_below_min_fails() {
        let mut config = config_with_text("hello");
        config.server.port = Some(80);
        assert!(validate(&config).unwrap_err().contains("Invalid port"));
    }

    #[test]
    fn port_at_min_is_ok() {
        let mut config = config_with_text("hello");
        config.server.port = Some(1024);
        assert!(validate(&config).is_ok());
    }

    #[test]
    fn invalid_blacklist_ip_fails() {
        let mut config = config_with_text("hello");
        config.security.blacklist = Some(vec!["not-an-ip".into()]);
        assert!(validate(&config).unwrap_err().contains("Invalid IP"));
    }

    #[test]
    fn valid_ipv6_in_whitelist() {
        let mut config = config_with_text("hello");
        config.security.whitelist = Some(vec!["::1".into()]);
        assert!(validate(&config).is_ok());
    }

    #[test]
    fn path_traversal_rejected() {
        let mut config = config_with_text("hello");
        config.content.text = None;
        config.content.from_file = Some("../etc/passwd".into());
        assert!(validate(&config).unwrap_err().contains(".."));
    }

    #[test]
    fn tor_and_port_forwarded_conflict() {
        let mut config = config_with_text("hello");
        config.server.tor = Some(true);
        config.server.port_forwarded = Some(true);
        assert!(validate(&config).unwrap_err().contains("--tor and --port-forwarded"));
    }

    #[test]
    fn cert_without_key_fails() {
        let mut config = config_with_text("hello");
        config.server.cert_path = Some("cert.pem".into());
        assert!(validate(&config).unwrap_err().contains("together"));
    }

    #[test]
    fn cert_with_insecure_http_fails() {
        let mut config = config_with_text("hello");
        config.server.cert_path = Some("cert.pem".into());
        config.server.key_path = Some("key.pem".into());
        config.server.insecure_http = Some(true);
        assert!(validate(&config).unwrap_err().contains("--insecure-http"));
    }

    #[test]
    fn cert_with_tor_fails() {
        let mut config = config_with_text("hello");
        config.server.cert_path = Some("cert.pem".into());
        config.server.key_path = Some("key.pem".into());
        config.server.tor = Some(true);
        assert!(validate(&config).unwrap_err().contains("Tor"));
    }
}
