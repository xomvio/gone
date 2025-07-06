use std::{collections::HashMap, str::FromStr, sync::{atomic::{AtomicBool, Ordering}, Arc}};
use tiny_http::{Header, Response, Server};
use clap::Parser;
use rand::{self, distr::{Distribution, Uniform}, Rng};
use chrono;
use serde::Deserialize;

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

#[derive(Deserialize)]
struct Config {
    port: Option<String>,
    content_type: Option<String>,
    server_name: Option<String>,
    from_file: Option<String>,
    text: Option<String>,
    endpoint: Option<String>,
    output: Option<String>,
    blacklist: Option<Vec<String>>
}

fn random_port() -> String {
    let mut rng = rand::rng();
    let port: u16 = rng.random();
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
    fn check(&mut self) -> bool {
        if self.blocked {
            return true;
        }

        let mut blocked = false;
        if self.visits.len() > 3 {
            blocked = true;
        }

        let last_visit = self.visits.last().unwrap();
        if last_visit.method == "POST" {
            blocked = true;
        }        
        
        self.blocked = blocked;
        blocked
    }
}

fn main() {
    let args = Args::parse();

    if args.from_file.is_none() && args.text.is_none() {        
        println!("You must specify either --from-file or --text\r\ntype \"sdhttpp --help\" for more info");
        return;
    }

    // one use flag
    let used = Arc::new(AtomicBool::new(false));
    
    let port = match args.port {
        Some(port) => port,
        None => random_port()
    };
    
    let endpoint = match args.endpoint {
        Some(endpoint) => endpoint,
        None => random_endpoint()
    };

    // tiny_http server
    let server = Server::http(format!("0.0.0.0:{}",port)).unwrap();

    // colors for terminal output
    let color_green = "\x1b[92m";
    let color_reset = "\x1b[0m";
    let color_yellow = "\x1b[93m";

    // key is ip address
    let mut visitors: HashMap<String, Visitor> = HashMap::new();


    println!("Server started \r\nport: {} \r\nendpoint: {}\r\n",port, endpoint);


    // Reject Response = Response::new(tiny_http::StatusCode(404), vec![Header::from_str(format!("Server: {}",args.server_name).as_str()).unwrap()], "".as_bytes(), None, None);
    for request in server.incoming_requests() {

        match visitors.get_mut(&request.remote_addr().unwrap().to_string()) {
            Some(visitor) => {
                visitor.visits.push(Visit {
                    datetime: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                    ip: request.remote_addr().unwrap().to_string(),
                    endpoint: request.url().to_string(),
                    method: request.method().as_str().to_string(),
                    version: request.http_version().to_string()
                });
            },
            None => {
                let first_visit = Visit {
                    datetime: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                    ip: request.remote_addr().unwrap().to_string(),
                    endpoint: request.url().to_string(),
                    method: request.method().as_str().to_string(),
                    version: request.http_version().to_string()
                };
                visitors.insert(request.remote_addr().unwrap().to_string(), Visitor {
                    visits: vec![first_visit],
                    blocked: false
                });
            }
        }

        let blocked =visitors.get_mut(&request.remote_addr().unwrap().to_string()).unwrap().check();

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


        //visitors.get_mut(&request.remote_addr().unwrap().to_string()).unwrap().check();

        // if visitor blocked, respond with 404
        if blocked {
            let resp = Response::new(tiny_http::StatusCode(404), vec![Header::from_str(format!("Server: {}",args.server_name).as_str()).unwrap()], "".as_bytes(), None, None);
            request.respond(resp).unwrap();
            continue;
        }

        if request.url() != ("/".to_string() + &endpoint) {
            let resp = Response::new(tiny_http::StatusCode(404), vec![Header::from_str(format!("Server: {}",args.server_name).as_str()).unwrap()], "".as_bytes(), None, None);
            request.respond(resp).unwrap();
            continue;
        }

        if !used.swap(true, Ordering::SeqCst) {
            // İlk ve tek erişim — mesajı göster

            println!("seen!");

            let msg = match args.from_file {
                Some(file_path) => std::fs::read_to_string(file_path).unwrap(),
                None => match args.text {
                    Some(msg) => msg,
                    None => "nothing".to_string()
                }
            };
            
            let content_type = Header::from_str(format!("Content-Type: {}",args.content_type).as_str()).unwrap();
            let server_hdr = Header::from_str(format!("Server: {}", args.server_name).as_str()).unwrap();
            let resp = Response::new(tiny_http::StatusCode(200), vec![content_type,server_hdr], msg.as_bytes(), Some(msg.len()), None);
            request.respond(resp).unwrap();
            
            return;
        } else {
            // diger tüm erişimlerde hata ver
            request.respond(Response::from_string("nothing")).unwrap();
        }
    }
}