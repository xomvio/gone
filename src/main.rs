mod config;
mod server;
mod visitor;
mod utils;

fn main() {
    // Set ring as default provider for rustls
    // Otherwise we will get a runtime error because arti_client also adds aws-lc-rs
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
