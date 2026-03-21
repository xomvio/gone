mod config;
mod constants;
mod server;
mod visitor;
mod utils;

fn main() {
    // When built with the `tor` feature, arti pulls in aws-lc-rs and sha2 pulls another one which conflicts
    // make ring as the default provider.
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    let config = match config::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    if let Err(e) = server::run(config) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
