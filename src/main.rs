use std::{collections::HashMap, str::FromStr, sync::{atomic::{AtomicBool, Ordering}, Arc}};
use tiny_http::{Header, Response, Server};
use clap::Parser;
use rand::{self, distr::{Distribution, Uniform}, Rng};
use chrono;
use serde::Deserialize;
use toml;

#[derive(Parser)]
struct Args {
    #[arg(short,long)]
    port:Option<String>,

    #[arg(short,long,default_value_t = String::from("text/html"))]
    content_type: String,

    #[arg(short,long,default_value_t = String::from("nginx"))]
    server_name: String,

    #[arg(short,long,value_name = "FILE")]
    from_file: Option<String>,

    #[arg(short,long)]
    text: Option<String>,

    #[arg(short,long)]
    endpoint: Option<String>,

    #[arg(short,long)]
    output: Option<String>
}

#[derive(Deserialize, Clone)]
struct Config {
    port: Option<String>,
    content_type: Option<String>,
    server_name: Option<String>,
    max_visits: Option<u32>,
    allowed_methods: Option<Vec<String>>,
    from_file: Option<String>,
    text: Option<String>,
    endpoint: Option<String>,
    output: Option<String>,
    blacklist: Option<Vec<String>>,
    whitelist: Option<Vec<String>>,
}


fn get_config() -> Config {
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

fn random_port() -> String {
    let mut rng = rand::rng();
    let port: u16 = rng.random_range(1024..=65535); // Use a valid port range
    port.to_string()
}

fn random_endpoint() -> String {
    let mut rng = rand::rng();
    let range = Uniform::new(97, 122).unwrap();
    
    let mut bytevec: [u8;64] = [0u8;64];
    for i in 0..bytevec.len() {
        bytevec[i] = range.sample(&mut rng);
    }
    
    String::from_utf8_lossy(&bytevec).to_string()
}

struct Visit {
    datetime: String,
    ip: String,
    endpoint: String,
    method: String,
    version: String,
}

struct Visitor {
    visits: Vec<Visit>,
    blocked: bool
}

impl Visitor {
    fn check(&mut self, config: &Config) -> bool {
        if self.blocked {
            return true;
        }

        let mut blocked = false;
        if self.visits.len() > 3 {
            blocked = true;
        }

        let last_visit = match self.visits.last() {
            Some(visit) => visit,
            None => return false, // No visits yet, not blocked
        };
        if last_visit.method == "POST" {
            blocked = true;
        }

        //check if whitelist is not empty
        
        
        self.blocked = blocked;
        blocked
    }
}

fn main() {
    //let args = Args::parse();

    let config = get_config();

    if config.from_file.is_none() && config.text.is_none() {        
        println!("You must specify either --from-file or --text\r\ntype \"sdhttpp --help\" for more info");
        return;
    }

    // one use flag
    let used = Arc::new(AtomicBool::new(false));
    
    let port = match &config.port {
        Some(port) => port,
        None => &random_port()
    };
    
    let endpoint = match &config.endpoint {
        Some(endpoint) => endpoint,
        None => &random_endpoint()
    };

    // tiny_http server
    let server = match Server::http(format!("0.0.0.0:{}", port)) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to start server: {}", e);
            std::process::exit(1);
        }
    };

    // colors for terminal output
    let color_green = "\x1b[92m";
    let color_reset = "\x1b[0m";
    let color_yellow = "\x1b[93m";

    // key is ip address
    let mut visitors: HashMap<String, Visitor> = HashMap::new();


    println!("Server started \r\nport: {} \r\nendpoint: {}\r\n",port, endpoint);


    // Reject Response = Response::new(tiny_http::StatusCode(404), vec![Header::from_str(format!("Server: {}",config.server_name).as_str()).unwrap()], "".as_bytes(), None, None);
    for request in server.incoming_requests() {

        let remote_addr = match request.remote_addr() {
            Some(addr) => addr.to_string(),
            None => {
                eprintln!("Could not get remote address");
                continue;
            }
        };
        
        match visitors.get_mut(&remote_addr) {
            Some(visitor) => {
                visitor.visits.push(Visit {
                    datetime: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                    ip: remote_addr.clone(),
                    endpoint: request.url().to_string(),
                    method: request.method().as_str().to_string(),
                    version: request.http_version().to_string()
                });
            },
            None => {
                let first_visit = Visit {
                    datetime: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                    ip: remote_addr.clone(),
                    endpoint: request.url().to_string(),
                    method: request.method().as_str().to_string(),
                    version: request.http_version().to_string()
                };
                visitors.insert(remote_addr.clone(), Visitor {
                    visits: vec![first_visit],
                    blocked: false
                });
            }
        }

        let blocked = if let Some(visitor) = visitors.get_mut(&remote_addr) {
            visitor.check(&config)
        } else {
            false
        };

        println!(r#"{color_green}Request{color_reset}
{color_yellow}DateTime:{color_reset}{}
{color_yellow}IP:{color_reset}{}
{color_yellow}Enpoint:{color_reset}{}
{color_yellow}Method:{color_reset}{}
{color_yellow}Version:{color_reset}{}
{}"#,
chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
request.remote_addr().unwrap().to_string(),
request.url(),
request.method().as_str(),
request.http_version(),
if blocked {"blocked\r\n"} else {""});

        // if visitor blocked, respond with 404
        if blocked || request.url() != ("/".to_string() + &endpoint) {
            let server_name = config.server_name.as_deref().unwrap_or("nginx");
            let server_header = format!("Server: {}", server_name);
            let header = match Header::from_str(&server_header) {
                Ok(h) => h,
                Err(e) => {
                    eprintln!("Failed to create header: {}", server_header);
                    std::process::exit(1);
                }
            };
            
            let resp = Response::new(
                tiny_http::StatusCode(404),
                vec![header],
                "".as_bytes(),
                None,
                None
            );
            if let Err(e) = request.respond(resp) {
                eprintln!("Failed to send response: {}", e);
            }
            continue;
        }

        if !used.swap(true, Ordering::SeqCst) {
            // İlk ve tek erişim. mesajı göster

            println!("seen!");

            let msg = match &config.from_file {
                Some(file_path) => match std::fs::read_to_string(file_path) {
                    Ok(content) => content,
                    Err(e) => {
                        eprintln!("Failed to read file: {}", e);
                        "Error reading file".to_string()
                    }
                },
                None => config.text.as_deref().unwrap_or("nothing").to_string()
            };
            
            let content_type = match Header::from_str(&format!("Content-Type: {}", config.content_type.as_deref().unwrap_or("text/html"))) {
                Ok(header) => header,
                Err(e) => {
                    eprintln!("Failed to create content type header: {}", config.content_type.as_deref().unwrap_or("text/html"));
                    std::process::exit(1);
                }
            };
            
            let server_hdr = match Header::from_str(&format!("Server: {}", config.server_name.as_deref().unwrap_or("nginx"))) {
                Ok(header) => header,
                Err(e) => {
                    eprintln!("Failed to create server header: {}", config.server_name.as_deref().unwrap_or("nginx"));
                    std::process::exit(1);
                }
            };
            
            let resp = Response::new(
                tiny_http::StatusCode(200),
                vec![content_type, server_hdr],
                msg.as_bytes(),
                Some(msg.len()),
                None
            );
            
            if let Err(e) = request.respond(resp) {
                eprintln!("Failed to send response: {}", e);
            }
            
            return;
        } else {
            // diger tüm erişimlerde hata ver
            if let Err(e) = request.respond(Response::from_string("nothing")) {
                eprintln!("Failed to send response: {}", e);
            }
        }
    }
}