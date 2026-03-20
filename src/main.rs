mod config;
mod server;
mod visitor;
mod utils;

fn main() {
    let config = config::load();
    server::run(config);
}
