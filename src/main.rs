mod config;
mod server;
mod visitor;
mod utils;

fn main() {
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    let config = config::load();
    server::run(config);
}
