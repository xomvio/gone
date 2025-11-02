mod config;
mod models;
mod server;
mod utils;

fn main() {
    let config = config::get();

    if config.from_file.is_none() && config.text.is_none() {        
        println!("You must specify either --from-file or --text\r\ntype \"sdhttpp --help\" for more info");
        return;
    }

    // Start the server
    server::run_server(config);
}