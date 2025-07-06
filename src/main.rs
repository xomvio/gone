use std::{str::FromStr, sync::{atomic::{AtomicBool, Ordering}, Arc}};
use tiny_http::{Header, Response, Server};
use clap::Parser;
use rand::{self, distr::{Distribution, Uniform}, Rng};
use chrono;

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


    println!("Server started \r\nport: {} \r\nendpoint: {}\r\n",port, endpoint);

    for request in server.incoming_requests() {

        println!(r#"{color_green}Request{color_reset}
{color_yellow}DateTime:{color_reset}{}
{color_yellow}IP:{color_reset}{}
{color_yellow}Enpoint:{color_reset}{}
"#,
chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
request.remote_addr().unwrap().to_string(),
request.url(),);

        if request.url() != ("/".to_string() + &endpoint) {
            request.respond(Response::from_string("nothing")).unwrap();
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
            /*let msg = r#"
F5 ÇEKME
Bu sayfa tek gösterimlik ve kendini yok edecek.
<h1>sa</h1>

            "#;*/
            
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
