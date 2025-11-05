mod config;
mod models;
mod server;
mod utils;

fn main() {
    let config = config::load();

    if config.content.from_file.is_none() && config.content.text.is_none() {        
        println!("You must specify either --from-file or --text\r\ntype \"sdhttpp --help\" for more info");
        return;
    }

    // Start the server
    server::run_server(config);
}