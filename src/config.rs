use clap::Parser;
use crate::models::Config;

#[derive(Parser)]
pub struct Args {
    #[arg(short,long)]
    pub port: Option<String>,

    #[arg(short,long,default_value_t = String::from("text/html"))]
    pub content_type: String,

    #[arg(short,long,default_value_t = String::from("nginx"))]
    pub server_name: String,

    #[arg(short,long,value_name = "FILE")]
    pub from_file: Option<String>,

    #[arg(short,long)]
    pub text: Option<String>,

    #[arg(short,long)]
    pub endpoint: Option<String>,

    #[arg(short,long)]
    pub output: Option<String>
}

pub fn get() -> Config {
    let args = Args::parse();

    let config_content = include_str!("../config.toml");
    let mut config: Config = toml::from_str(config_content).unwrap_or_else(|e| {
        eprintln!("Error parsing config.toml: {}", e);
        std::process::exit(1);
    });

    if args.port.is_some() {
        config.port = args.port;
    }
    if args.from_file.is_some() {
        config.from_file = args.from_file;
    }
    if args.text.is_some() {
        config.text = args.text;
    }
    if args.endpoint.is_some() {
        config.endpoint = args.endpoint;
    }
    if args.output.is_some() {
        config.output = args.output;
    }
    
    config
}