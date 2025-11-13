use rand::distr::{Alphanumeric};
use rand::Rng;

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
