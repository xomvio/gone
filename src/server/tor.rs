use std::fs::File;
use std::io::{BufWriter, Write};
use std::sync::Mutex;

use arti_client::config::onion_service::OnionServiceConfigBuilder;
use arti_client::config::{BoolOrAuto, CfgPath};
use arti_client::{TorClient, TorClientConfig};
use tor_config::ExplicitOrAuto;
use tor_keymgr::config::ArtiKeystoreKind;
use futures::StreamExt;
use safelog::DisplayRedacted;
use tokio::io::AsyncWriteExt;
use tokio_util::io::SyncIoBridge;
use tor_cell::relaycell::msg::Connected;
use tor_hsservice::handle_rend_requests;

use crate::{config::Config, constants, utils};

use super::{handle_connection, HandleResult};

pub fn run(config: Config) -> Result<(), String> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .map_err(|e| format!("Failed to create tokio runtime: {}", e))?;

    let result = rt.block_on(run_async(config));

    // Gracefully shut down the runtime so Tor circuit tasks can finish
    // delivering buffered data before the process exits.
    rt.shutdown_timeout(std::time::Duration::from_secs(10));
    result
}

async fn run_async(config: Config) -> Result<(), String> {
    let endpoint = config.server.endpoint.clone().unwrap_or_else(utils::random_endpoint);
    let expected_url = format!("/{}", endpoint);
    let server_name = config.server.server_name.as_deref().unwrap_or(constants::DEFAULT_SERVER_NAME).to_string();
    let log_file: Mutex<Option<BufWriter<File>>> = Mutex::new(utils::open_log_file(&config)?);

    println!("Bootstrapping Tor... (this may take a moment)");

    let tmp_dir = tempfile::tempdir().unwrap();
    let mut config_builder = TorClientConfig::builder();
    config_builder
        .storage()
        .cache_dir(CfgPath::new(tmp_dir.path().join("cache").to_string_lossy().into_owned()))
        .state_dir(CfgPath::new(tmp_dir.path().join("state").to_string_lossy().into_owned()))
        .keystore()
        .enabled(BoolOrAuto::Explicit(true))
        .primary()
        .kind(ExplicitOrAuto::Explicit(ArtiKeystoreKind::Ephemeral));

    let tor_config = config_builder
        .build()
        .map_err(|e| format!("Failed to build Tor config: {}", e))?;

    let tor_client = TorClient::create_bootstrapped(tor_config)
        .await
        .map_err(|e| format!("Failed to bootstrap Tor: {}", e))?;

    let svc_config = OnionServiceConfigBuilder::default()
        .nickname(
            utils::random_tor_nickname().parse()
                .map_err(|e| format!("Failed to parse nickname: {}", e))?,
        )
        .build()
        .map_err(|e| format!("Failed to build onion service config: {}", e))?;

    let (onion_service, rend_requests) = tor_client
        .launch_onion_service(svc_config)
        .map_err(|e| format!("Failed to launch onion service: {}", e))?
        .ok_or_else(|| "Onion service unavailable (no keystore configured?)".to_string())?;

    // Wait until the onion address is available
    let onion_addr = loop {
        if let Some(addr) = onion_service.onion_address() {
            break addr;
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    };

    let hash = match &config.content.from_file {
        Some(path) => utils::sha256_file(path).unwrap_or_else(|| "hash error".to_string()),
        None => match &config.content.stdin_data {
            Some(data) => utils::sha256_bytes(data),
            None => {
                let text = config.content.text.as_deref().unwrap_or("No content");
                utils::sha256_text(text)
            }
        }
    };

    println!(
        "Server started\nonion: {}\nendpoint: {}\nHash: {}\n",
        onion_addr.display_unredacted(),
        expected_url,
        hash
    );

    let mut stream_requests = handle_rend_requests(rend_requests);

    loop {
        let stream_req = match stream_requests.next().await {
            Some(s) => s,
            None => return Err("Onion service stream ended unexpectedly".to_string()),
        };

        let data_stream = match stream_req.accept(Connected::new_empty()).await {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to accept stream: {}", e);
                continue;
            }
        };

        // No explicit read timeout for Tor: the onion address is secret,
        // Tor circuits have their own timeouts, and SyncIoBridge cannot be
        // cancelled from outside a blocking context.
        //
        // Bridge async stream → sync Read+Write for handle_connection
        let (result, mut data_stream) = tokio::task::block_in_place(|| {
            let mut sync_stream = SyncIoBridge::new(data_stream);
            let result = handle_connection(
                &mut sync_stream,
                "tor".to_string(),
                &expected_url,
                &server_name,
                &config,
                &log_file,
            );
            (result, sync_stream.into_inner())
        });

        // Always flush and close the stream so the client receives the response
        let _ = data_stream.flush().await;
        let _ = data_stream.shutdown().await;

        match result {
            HandleResult::Continue => continue,
            HandleResult::Served | HandleResult::ServeError => {
                if let Ok(mut lf) = log_file.lock() && let Some(f) = lf.as_mut() {
                    let _ = f.flush(); 
                }
                
                // Keep tor_client alive while Tor relays deliver the data
                // Also sleep for a random seconds to mitigate correlation attacks
                let sleep_secs = rand::random_range(5..60);
                println!("Sending data... waiting a random interval before shutdown to mitigate correlation attacks.");
                tokio::time::sleep(tokio::time::Duration::from_secs(sleep_secs)).await;
                return if matches!(result, HandleResult::Served) {
                    Ok(())
                } else {
                    Err("Failed to serve content".to_string())
                };
            }
        }
    }
}
