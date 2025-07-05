use std::{str::FromStr, sync::{atomic::{AtomicBool, Ordering}, Arc}};
use tiny_http::{Header, Response, Server};
use clap::{Parser};
//use random::{self, Source};
use rand::{self, distr::{uniform::UniformChar, Distribution, Uniform}, rng, Rng, RngCore};

#[derive(Parser)]
struct Args {
    #[arg(short,long)]
    port:Option<String>,

    #[arg(short,long,default_value_t = String::from("text/html"))]
    content_type: String,

    #[arg(short,long,default_value_t = String::from("nginx"))]
    server_name: String,

    #[arg(short,long)]
    from_file: Option<String>,

    #[arg(short,long)]
    message: Option<String>,

    #[arg(short,long)]
    endpoint: Option<String>,

}

fn random_port() -> String {
    //let mut rand = random::default(316684654354);
    let mut rng = rand::rng();
    //let mut chars:Vec<char> = vec![];
    //println!("{}",rng.random::<char>());
    let port: u16 = rng.random();
    port.to_string()
}

fn random_endpoint() -> String {
    //let mut rand = random::default(68686546463);
    let mut rng = rand::rng();
    let range = Uniform::new(97, 122).unwrap();
    
    let mut bytevec: [u8;64] = [0u8;64];
    for i in 0..bytevec.len() {
        bytevec[i] = range.sample(&mut rng);
    }
    range.sample_iter(rng);
    //rng.fill_bytes(&mut bytevec);
    for byte in bytevec {
        print!("{}",byte as char);
    }
    "".to_string()
}

fn main() {
    random_endpoint();
    return;
    let mut args = Args::parse();

    // one use flag
    let used = Arc::new(AtomicBool::new(false));
    
    let port = match args.port {
        Some(port)=> port,
        None => random_port()
    };
    
    let endpoint = match args.endpoint {
        Some(endpoint)=>endpoint,
        None => random_endpoint()
    };

    // tiny_http server
    let server = Server::http(format!("0.0.0.0:{}",port)).unwrap();

    println!("Server started \r\nport: {} \r\nendpoint: {}",port, endpoint);

    for request in server.incoming_requests() {
        println!("{}", request.url());
        //let used = used.clone();

        if request.url() != "/2FMUF3KwE8rkLsKF02yp0QCp1" {
            request.respond(Response::from_string("nothing")).unwrap();
            continue;
        }

        if !used.swap(true, Ordering::SeqCst) {
            // İlk ve tek erişim — mesajı göster

            println!("seen!");
            let msg = r#"
F5 ÇEKME
Bu sayfa tek gösterimlik ve kendini yok edecek.
<h1>sa</h1>

            "#;
            
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
