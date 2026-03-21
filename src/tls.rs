use std::io::BufReader;
use std::fs::File;
use std::sync::Arc;

use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};

use crate::config::Config;

/// Builds a rustls ServerConfig. Loads cert/key from PEM files if paths are provided;
/// otherwise generates a self-signed certificate with rcgen.
pub fn make_tls_config(config: &Config) -> Arc<rustls::ServerConfig> {
    let (certs, key) = match (&config.server.cert_path, &config.server.key_path) {
        (Some(cert_path), Some(key_path)) => load_from_pem(cert_path, key_path),
        _ => generate_self_signed(),
    };

    let tls_config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .unwrap_or_else(|e| {
            eprintln!("Failed to build TLS config: {}", e);
            std::process::exit(1);
        });

    Arc::new(tls_config)
}

fn generate_self_signed() -> (Vec<CertificateDer<'static>>, PrivateKeyDer<'static>) {
    let subject_alt_names = vec!["localhost".to_string(), "127.0.0.1".to_string()];
    let certified_key = rcgen::generate_simple_self_signed(subject_alt_names)
        .unwrap_or_else(|e| {
            eprintln!("Failed to generate self-signed certificate: {}", e);
            std::process::exit(1);
        });

    let cert_der = CertificateDer::from(certified_key.cert.der().to_vec());
    let key_der = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(
        certified_key.key_pair.serialize_der(),
    ));

    (vec![cert_der], key_der)
}

fn load_from_pem(
    cert_path: &str,
    key_path: &str,
) -> (Vec<CertificateDer<'static>>, PrivateKeyDer<'static>) {
    let cert_file = File::open(cert_path).unwrap_or_else(|e| {
        eprintln!("Failed to open cert file '{}': {}", cert_path, e);
        std::process::exit(1);
    });
    let mut cert_reader = BufReader::new(cert_file);
    let certs: Vec<CertificateDer<'static>> = rustls_pemfile::certs(&mut cert_reader)
        .collect::<Result<_, _>>()
        .unwrap_or_else(|e| {
            eprintln!("Failed to parse cert file '{}': {}", cert_path, e);
            std::process::exit(1);
        });

    if certs.is_empty() {
        eprintln!("Error: No certificates found in '{}'", cert_path);
        std::process::exit(1);
    }

    let key_file = File::open(key_path).unwrap_or_else(|e| {
        eprintln!("Failed to open key file '{}': {}", key_path, e);
        std::process::exit(1);
    });
    let mut key_reader = BufReader::new(key_file);
    let key = rustls_pemfile::private_key(&mut key_reader)
        .unwrap_or_else(|e| {
            eprintln!("Failed to parse key file '{}': {}", key_path, e);
            std::process::exit(1);
        })
        .unwrap_or_else(|| {
            eprintln!("Error: No private key found in '{}'", key_path);
            std::process::exit(1);
        });

    (certs, key)
}
