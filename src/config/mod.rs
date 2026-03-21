use std::borrow::Cow;
const DEFAULT_CONFIG: &str = include_str!("../../default-config.toml");

mod load;
mod args;
mod validate;
mod defaults;

pub fn load() -> Config {
    load::load()
}

pub fn validate(config: &Config) {
    validate::validate(config);
}

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
    pub insecure_http: Option<bool>,
    pub tor: Option<bool>,
    pub port_forwarded: Option<bool>,
    pub cert_path: Option<String>,
    pub key_path: Option<String>,
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
