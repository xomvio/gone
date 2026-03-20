use std::net::SocketAddr;
use rand::distr::Alphanumeric;
use rand::Rng;
use chrono;


pub fn random_port() -> u16 {
    let mut rng = rand::rng();
    rng.random_range(1024..=65535)
}

pub fn random_endpoint() -> String {
    let rng = rand::rng();
    rng.sample_iter(&Alphanumeric)
        .take(64)
        .map(char::from)
        .collect()
}

pub fn now_str() -> String {
    chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

pub fn cow_str_to_str<'a>(cow: &'a Option<std::borrow::Cow<'static, str>>, default: &'static str) -> &'a str {
    cow.as_deref().unwrap_or(default)
}

pub fn extract_ip(request: &tiny_http::Request) -> Option<String> {
    request.remote_addr().map(|addr| {
        let s = addr.to_string();
        s.parse::<SocketAddr>()
            .map(|a| a.ip().to_string())
            .unwrap_or(s)
    })
}
