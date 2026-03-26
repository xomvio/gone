# sdhttpp - Self-Destructing HTTP(s) Provider

A single-use HTTPS server that serves content just once, then shuts down.

With built-in Tor support, both sender and receiver can stay anonymous.

No accounts, no cloud, no third-party.

## Use Cases
- Transfer sensitive documents to a journalist over Tor
- Share a file with a friend without uploading to any service
- Send files to other devices in your local network

## Quick Start
```bash
# Serve a file over HTTPS
sdhttpp --from-file ./document.pdf

# Serve a text
sdhttpp --text "meet me in central park"

# Serve a file over Tor
sdhttpp --tor --from-file ./document.pdf

# Pipe from stdin (e.g., with encryption)
age -e -r <recipient-key> secret.pdf | sdhttpp --from-file - --stdin-filename secret.pdf.age
```

## After Running
After running sdhttpp you'll see:
```bash
Server started
port: 47832
endpoint: /a8Fx2k...
Hash: 3d2e...
```

Share the URL with recipient:

```http://<ip>:<port>/<endpoint>```

or over tor
```http://<onion-address>.onion/<endpoint>```

They open it once → content is delivered → server shuts down.

## Receiving Content
The receiver just opens the URL in their browser. That's it.
- For HTTPS: if manual certificate files did not given, the browser will show a certificate warning (self-signed). This is expected.
- For Tor: Receiver must use Tor Browser.
- For insecure HTTP: any browser works, no warnings.

## Examples

> **Note:** Without `--tor` or `--port-forwarded`, the receiver needs 
> direct access to your IP (e.g., public IP or same network).

### Basic: share over local network
```sdhttpp --insecure-http --from-file ./photo.jpg```

Just open `http://<local-ip>:<port>/<endpoint>` from any device on the same network and the download will start.

### With self-signed HTTPS (default)
`sdhttpp --text "secret message"`

### With manual HTTPS
`sdhttpp --text "secret message" --cert-path "/path/to/cert" --key-path "/path/to/key"`

### With built-in Tor (requires tor version)
`sdhttpp --tor --text "secret message"`

### Behind a reverse proxy or VPN
`sdhttpp --port-forwarded --from-file ./data.csv`

### Pipe from stdin
```bash
# Encrypt and serve in one command
age -e -r <recipient-key> secret.pdf | sdhttpp --from-file - --stdin-filename secret.pdf.age

# Pipe any command output
tar czf - ./folder | sdhttpp --from-file - --stdin-filename folder.tar.gz
```
`--from-file -` reads from stdin. `--stdin-filename` sets the download filename for the receiver.

### With custom settings
`sdhttpp --port 9191 --endpoint mylink --from-file ./file.zip`

## Usage
<!-- generated from sdhttpp --help -->
```bash
Usage: sdhttpp [OPTIONS]

Options:
  -p, --port <PORT>                Port to listen on (1024-65535)
      --content-type <TYPE>        Content-Type header for the response (e.g., "text/plain", "text/html")
      --server-name <NAME>         Server header value (default: "nginx")
      --from-file <FILE>           Path to file to serve (alternative to --text)
      --text <TEXT>                Text to serve directly (alternative to --from-file)
      --endpoint <PATH>            Custom endpoint path (must start with /)
      --output <FILE>              Path to output log file (default: stdout)
      --allowed-methods <METHODS>  Allowed HTTP methods, comma-separated (e.g., GET,POST)
      --blacklist <IPS>            IP addresses to always block, comma-separated
      --whitelist <IPS>            IP addresses to allow exclusively, comma-separated
      --insecure-http              Disable TLS and use plain HTTP (HTTPS is the default)
      --tor                        Route through Tor (starts an onion service via arti). Requires building with `--features tor`
      --port-forwarded             Listen only on 127.0.0.1 (for use with external port forwarding like tor-daemon or nginx)
      --cert-path <FILE>           Path to TLS certificate file (PEM format). Requires --key-path
      --key-path <FILE>            Path to TLS private key file (PEM format). Requires --cert-path
      --stdin-filename <NAME>      Filename for Content-Disposition header when reading from stdin (requires --from-file -)
      --quiet                      Suppress request logging to stdout (server info and hash still shown)
      --generate-config            Generate a default config file and exit
  -c, --config <CONFIG>            Path to config file (default: config.toml in current directory) [default: config.toml]
  -h, --help                       Print help
  -V, --version                    Print version
```

## Installation / Building
> Compilation from source requires [Rust](https://rustup.rs/) toolchain.
> 
### Without Tor:
Download latest binary from releases or compile from source:

```bash
cargo build --release
# Binary: target/release/sdhttpp
./sdhttpp --text "secret message"
```

### With Tor Support:
Download latest binary(Tor version) from releases or compile from source:
```bash
cargo build --release --features tor
# Binary: target/release/sdhttpp
./sdhttpp --tor --text "secret message"
```

## Security

### What sdhttpp does
- Serves content exactly once, then shuts down. Minimal attack surface.
- Generates a random 64-character endpoint which acts as a shared secret
- HTTPS by default with self-signed certificate
- Tor mode hides both sender's and receiver's IP addresses
- Prints SHA-256 hash. So the receiver can verify content integrity
- Disguises itself as nginx to casual port scanners

### What sdhttpp does **NOT** do
- **No end-to-end encryption.** HTTPS protects the transport layer only. If your threat model includes state-level actors or compromised CAs, consider encrypting your content before serving it, even over Tor.
- No recipient authentication. Anyone with the correct URL gets the content. Share the URL only through a secure channel (encrypted messaging, or even pen and paper).

### Recommendations
- For sensitive data: always use --tor
- For maximum security: encrypt content with [age](https://github.com/FiloSottile/age) or GPG before serving:
  ```bash
  age -e -r <recipient-key> secret.pdf | sdhttpp --tor --from-file - --stdin-filename secret.pdf.age
  ```
- For better Tor stability: consider using the Tor daemon with --port-forwarded instead of built-in Arti (which is official but experimental)

### About --insecure-http
Disables TLS entirely. Use this only when:
- Transferring between local devices
- Using Tor (--tor disables TLS automatically)
- Using I2P or other overlay networks
- With Custom encryption pipelines

## Configuration
You can configure sdhttpp in command-line arguments or with a config file.

Generating a default config file:
```bash
sdhttpp --generate-config
```
Then edit config.toml as you need. Command-line arguments will override config file values.

## License
MIT. See [LICENSE](LICENSE)
