use rand::{self, distr::{Distribution, Uniform}, Rng};

pub fn random_port() -> String {
    let mut rng = rand::rng();
    let port: u16 = rng.random_range(1024..=65535);
    port.to_string()
}

pub fn random_endpoint() -> String {
    let mut rng = rand::rng();
    let range = Uniform::new(97, 122).unwrap();
    
    let mut bytevec: [u8; 64] = [0u8; 64];
    for i in 0..bytevec.len() {
        bytevec[i] = range.sample(&mut rng);
    }
    
    String::from_utf8_lossy(&bytevec).to_string()
}
