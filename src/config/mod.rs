const DEFAULT_CONFIG: &str = include_str!("../../default-config.toml");

mod load;
mod args;
mod validate;
mod defaults;

pub fn load() -> Result<Config, String> {
    load::load()
}

pub fn validate(config: &Config) -> Result<(), String> {
    validate::validate(config)
}

#[derive(serde::Deserialize, Clone, Debug, Default)]
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
    pub content_type: Option<String>,
    pub server_name: Option<String>,
    pub endpoint: Option<String>,
    pub output: Option<String>,
    pub insecure_http: Option<bool>,
    pub tor: Option<bool>,
    pub port_forwarded: Option<bool>,
    pub cert_path: Option<String>,
    pub key_path: Option<String>,
    pub quiet: Option<bool>,
}

#[derive(serde::Deserialize, Clone, Debug)]
pub struct ContentConfig {
    pub text: Option<String>,
    pub from_file: Option<String>,
}

#[derive(serde::Deserialize, Clone, Debug)]
pub struct SecurityConfig {
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

// Tests __________________________________________

#[cfg(test)]
mod tests {
    use super::*;

    fn security_config(
        methods: Option<Vec<&str>>,
        whitelist: Option<Vec<&str>>,
        blacklist: Option<Vec<&str>>,
    ) -> SecurityConfig {
        SecurityConfig {
            allowed_methods: methods.map(|m| m.into_iter().map(String::from).collect()),
            whitelist: whitelist.map(|w| w.into_iter().map(String::from).collect()),
            blacklist: blacklist.map(|b| b.into_iter().map(String::from).collect()),
        }
    }

    #[test]
    fn method_allowed_when_no_restriction() {
        let sec = security_config(None, None, None);
        assert!(sec.is_method_allowed("GET"));
        assert!(sec.is_method_allowed("POST"));
        assert!(sec.is_method_allowed("DELETE"));
    }

    #[test]
    fn method_allowed_only_listed() {
        let sec = security_config(Some(vec!["GET"]), None, None);
        assert!(sec.is_method_allowed("GET"));
        assert!(!sec.is_method_allowed("POST"));
    }

    #[test]
    fn method_allowed_case_insensitive() {
        let sec = security_config(Some(vec!["GET"]), None, None);
        assert!(sec.is_method_allowed("get"));
        assert!(sec.is_method_allowed("Get"));
    }

    #[test]
    fn ip_allowed_when_no_lists() {
        let sec = security_config(None, None, None);
        assert!(sec.is_ip_allowed("1.2.3.4"));
    }

    #[test]
    fn ip_blocked_by_blacklist() {
        let sec = security_config(None, None, Some(vec!["1.2.3.4"]));
        assert!(!sec.is_ip_allowed("1.2.3.4"));
        assert!(sec.is_ip_allowed("5.6.7.8"));
    }

    #[test]
    fn ip_whitelist_only_allows_listed() {
        let sec = security_config(None, Some(vec!["1.2.3.4"]), None);
        assert!(sec.is_ip_allowed("1.2.3.4"));
        assert!(!sec.is_ip_allowed("5.6.7.8"));
    }

    #[test]
    fn ip_whitelist_takes_priority_over_blacklist() {
        let sec = security_config(None, Some(vec!["1.2.3.4"]), Some(vec!["1.2.3.4"]));
        // Whitelist active → only whitelist matters
        assert!(sec.is_ip_allowed("1.2.3.4"));
        assert!(!sec.is_ip_allowed("5.6.7.8"));
    }

    #[test]
    fn ip_empty_whitelist_falls_through_to_blacklist() {
        let sec = security_config(None, Some(vec![]), Some(vec!["1.2.3.4"]));
        // Empty whitelist = not active → blacklist applies
        assert!(!sec.is_ip_allowed("1.2.3.4"));
        assert!(sec.is_ip_allowed("5.6.7.8"));
    }
}
