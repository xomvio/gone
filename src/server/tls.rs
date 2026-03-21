use std::io::BufReader;
use std::fs::File;
use std::sync::Arc;

use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};

use crate::config::Config;

/// Builds a rustls ServerConfig. Loads cert/key from PEM files if paths are provided;
/// otherwise generates a self-signed certificate with rcgen.
pub fn make_tls_config(config: &Config) -> Result<Arc<rustls::ServerConfig>, String> {
    let (certs, key) = match (&config.server.cert_path, &config.server.key_path) {
        (Some(cert_path), Some(key_path)) => load_from_pem(cert_path, key_path)?,
        _ => generate_self_signed()?,
    };

    let mut tls_config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|e| format!("Failed to build TLS config: {}", e))?;

    tls_config.alpn_protocols = vec![b"http/1.1".to_vec()];

    Ok(Arc::new(tls_config))
}

fn generate_self_signed() -> Result<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>), String> {
    let subject_alt_names = vec!["localhost".to_string(), "127.0.0.1".to_string()];
    let certified_key = rcgen::generate_simple_self_signed(subject_alt_names)
        .map_err(|e| format!("Failed to generate self-signed certificate: {}", e))?;

    let cert_der = CertificateDer::from(certified_key.cert.der().to_vec());
    let key_der = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(
        certified_key.key_pair.serialize_der(),
    ));

    Ok((vec![cert_der], key_der))
}

fn load_from_pem(
    cert_path: &str,
    key_path: &str,
) -> Result<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>), String> {
    let cert_file = File::open(cert_path)
        .map_err(|e| format!("Failed to open cert file '{}': {}", cert_path, e))?;
    let mut cert_reader = BufReader::new(cert_file);
    let certs: Vec<CertificateDer<'static>> = rustls_pemfile::certs(&mut cert_reader)
        .collect::<Result<_, _>>()
        .map_err(|e| format!("Failed to parse cert file '{}': {}", cert_path, e))?;

    if certs.is_empty() {
        return Err(format!("No certificates found in '{}'", cert_path));
    }

    let key_file = File::open(key_path)
        .map_err(|e| format!("Failed to open key file '{}': {}", key_path, e))?;
    let mut key_reader = BufReader::new(key_file);
    let key = rustls_pemfile::private_key(&mut key_reader)
        .map_err(|e| format!("Failed to parse key file '{}': {}", key_path, e))?
        .ok_or_else(|| format!("No private key found in '{}'", key_path))?;

    Ok((certs, key))
}
