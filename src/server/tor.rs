use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Write};

use arti_client::config::onion_service::OnionServiceConfigBuilder;
use arti_client::{TorClient, TorClientConfig};
use futures::StreamExt;
use safelog::DisplayRedacted;
use tokio::io::AsyncWriteExt;
use tokio_util::io::SyncIoBridge;
use tor_cell::relaycell::msg::Connected;
use tor_hsservice::handle_rend_requests;

use crate::{config::Config, utils, visitor::Visitor};

use super::{handle_connection, HandleResult};

pub fn run(config: Config) -> ! {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap_or_else(|e| {
            eprintln!("Failed to create tokio runtime: {}", e);
            std::process::exit(1);
        });

    let exit_code = rt.block_on(run_async(config));

    // Gracefully shut down the runtime so Tor circuit tasks can finish
    // delivering buffered data before the process exits.
    rt.shutdown_timeout(std::time::Duration::from_secs(10));
    std::process::exit(exit_code);
}

async fn run_async(config: Config) -> i32 {
    let endpoint = config.server.endpoint.clone().unwrap_or_else(utils::random_endpoint);
    let expected_url = format!("/{}", endpoint);
    let server_name = utils::cow_str_to_str(&config.server.server_name, "nginx").to_string();
    let mut log_file: Option<BufWriter<File>> = utils::open_log_file(&config);
    let mut visitors: HashMap<String, Visitor> = HashMap::new();

    println!("Bootstrapping Tor... (this may take a moment)");

    let tor_client = TorClient::create_bootstrapped(TorClientConfig::default())
        .await
        .unwrap_or_else(|e| {
            eprintln!("Failed to bootstrap Tor: {}", e);
            std::process::exit(1);
        });

    let svc_config = OnionServiceConfigBuilder::default()
        .nickname(
            "sdhttpp".parse().unwrap_or_else(|e| {
                eprintln!("Failed to parse nickname: {}", e);
                std::process::exit(1);
            }),
        )
        .build()
        .unwrap_or_else(|e| {
            eprintln!("Failed to build onion service config: {}", e);
            std::process::exit(1);
        });

    let (onion_service, rend_requests) = tor_client
        .launch_onion_service(svc_config)
        .unwrap_or_else(|e| {
            eprintln!("Failed to launch onion service: {}", e);
            std::process::exit(1);
        })
        .unwrap_or_else(|| {
            eprintln!("Onion service unavailable (no keystore configured?)");
            std::process::exit(1);
        });

    // Wait until the onion address is available
    let onion_addr = loop {
        if let Some(addr) = onion_service.onion_address() {
            break addr;
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    };

    println!(
        "Server started\nonion: {}\nendpoint: {}\n",
        onion_addr.display_unredacted(),
        expected_url
    );

    let mut stream_requests = handle_rend_requests(rend_requests);

    loop {
        let stream_req = match stream_requests.next().await {
            Some(s) => s,
            None => {
                eprintln!("Onion service stream ended unexpectedly");
                return 1;
            }
        };

        let data_stream = match stream_req.accept(Connected::new_empty()).await {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to accept stream: {}", e);
                continue;
            }
        };

        // Bridge async stream → sync Read+Write for handle_connection
        let (result, mut data_stream) = tokio::task::block_in_place(|| {
            let mut sync_stream = SyncIoBridge::new(data_stream);
            let result = handle_connection(
                &mut sync_stream,
                "tor".to_string(),
                &expected_url,
                &server_name,
                &config,
                &mut visitors,
                &mut log_file,
            );
            (result, sync_stream.into_inner())
        });

        // Always flush and close the stream so the client receives the response
        let _ = data_stream.flush().await;
        let _ = data_stream.shutdown().await;

        match result {
            HandleResult::Continue => continue,
            HandleResult::Served | HandleResult::ServeError => {
                if let Some(f) = &mut log_file { let _ = f.flush(); }
                // Keep tor_client alive while Tor relays deliver the data
                // Also sleep for a random seconds to mitigate correlation attacks
                let sleep_secs = rand::random_range(5..60);
                println!("sending data...\nAlso waiting for {} (random) seconds before shutdown server to avoid correlation attacks.",sleep_secs);
                tokio::time::sleep(tokio::time::Duration::from_secs(sleep_secs)).await;
                return if matches!(result, HandleResult::Served) { 0 } else { 1 };
            }
        }
    }
}
