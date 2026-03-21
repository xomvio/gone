use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Port to listen on (1024-65535)
    #[arg(short, long)]
    pub port: Option<String>,

    /// Content-Type header for the response (e.g., "text/plain", "text/html")
    #[arg(long, value_name = "TYPE")]
    pub content_type: Option<String>,

    /// Server header value (default: "nginx")
    #[arg(long, value_name = "NAME")]
    pub server_name: Option<String>,

    /// Path to file to serve (alternative to --text)
    #[arg(long, value_name = "FILE")]
    pub from_file: Option<String>,

    /// Text to serve directly (alternative to --from-file)
    #[arg(long, value_name = "TEXT")]
    pub text: Option<String>,

    /// Custom endpoint path (must start with /)
    #[arg(long, value_name = "PATH")]
    pub endpoint: Option<String>,

    /// Path to output log file (default: stdout)
    #[arg(long, value_name = "FILE")]
    pub output: Option<String>,

    /// Allowed HTTP methods, comma-separated (e.g., GET,POST)
    #[arg(long, value_name = "METHODS", value_delimiter = ',')]
    pub allowed_methods: Option<Vec<String>>,

    /// IP addresses to always block, comma-separated
    #[arg(long, value_name = "IPS", value_delimiter = ',')]
    pub blacklist: Option<Vec<String>>,

    /// IP addresses to allow exclusively, comma-separated
    #[arg(long, value_name = "IPS", value_delimiter = ',')]
    pub whitelist: Option<Vec<String>>,

    /// Disable TLS and use plain HTTP (HTTPS is the default).
    #[arg(long)]
    pub insecure_http: bool,

    /// Route through Tor (starts an onion service via arti).
    /// Requires building with `--features tor`.
    #[arg(long)]
    pub tor: bool,

    /// Listen only on 127.0.0.1 (for use with external port forwarding like tor-daemon or nginx).
    #[arg(long)]
    pub port_forwarded: bool,

    /// Path to TLS certificate file (PEM format). Requires --key-path.
    #[arg(long, value_name = "FILE")]
    pub cert_path: Option<String>,

    /// Path to TLS private key file (PEM format). Requires --cert-path.
    #[arg(long, value_name = "FILE")]
    pub key_path: Option<String>,

    /// Generate a default config file and exit
    #[arg(long)]
    pub generate_config: bool,

    /// Path to config file (default: config.toml in current directory)
    #[arg(short, long, default_value = "config.toml")]
    pub config: String,
}