mod config;
mod http;
mod server;
mod tls;
mod visitor;
mod utils;

fn main() {
    let config = config::load();
    server::run(config);
}
