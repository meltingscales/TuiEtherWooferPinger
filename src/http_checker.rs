use crate::stats::Stats;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Start an async HTTP checking task for a specific IP address
pub async fn start_http_task(
    ip: IpAddr,
    port: u16,
    stats: Arc<RwLock<HashMap<IpAddr, Stats>>>,
    mut shutdown: tokio::sync::watch::Receiver<bool>,
) {
    // Create HTTP client with timeout
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .redirect(reqwest::redirect::Policy::none()) // Don't follow redirects
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to create HTTP client for {}: {}", ip, e);
            return;
        }
    };

    let url = format!("http://{}:{}", ip, port);
    let mut interval = tokio::time::interval(Duration::from_secs(1));

    loop {
        tokio::select! {
            _ = interval.tick() => {
                // Measure request time
                let start = Instant::now();
                let result = client.get(&url).send().await;
                let duration = start.elapsed();

                // Extract stats from response
                let (status_code, content_size, error) = match result {
                    Ok(resp) => {
                        let status = resp.status().as_u16();
                        let size = resp.content_length();
                        (Some(status), size, None)
                    }
                    Err(e) => {
                        let error_msg = if e.is_timeout() {
                            "Request timeout".to_string()
                        } else if e.is_connect() {
                            "Connection refused".to_string()
                        } else {
                            format!("{}", e)
                        };
                        (None, None, Some(error_msg))
                    }
                };

                // Update statistics
                {
                    let mut stats_lock = stats.write();
                    if let Some(Stats::Http(host_stats)) = stats_lock.get_mut(&ip) {
                        host_stats.update(status_code, duration, content_size, error);
                    }
                }
            }
            _ = shutdown.changed() => {
                // Graceful shutdown
                break;
            }
        }
    }
}
