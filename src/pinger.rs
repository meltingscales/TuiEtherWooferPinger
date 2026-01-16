use crate::stats::Stats;
use parking_lot::RwLock;
use rand::Rng;
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;
use surge_ping::{Client, Config, PingIdentifier, PingSequence};

/// Start an async ping task for a specific IP address
pub async fn start_ping_task(
    ip: IpAddr,
    stats: Arc<RwLock<HashMap<IpAddr, Stats>>>,
    mut shutdown: tokio::sync::watch::Receiver<bool>,
) {
    // Create ping client
    let config = Config::default();
    let client = match Client::new(&config) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to create ping client for {}: {}", ip, e);
            return;
        }
    };

    // Create pinger with random identifier
    let ping_id = rand::thread_rng().gen::<u16>();
    let mut pinger = client.pinger(ip, PingIdentifier(ping_id)).await;

    let mut interval = tokio::time::interval(Duration::from_secs(1));
    let mut seq = 0u16;
    let payload = [0u8; 56];

    loop {
        tokio::select! {
            _ = interval.tick() => {
                // Send ping and wait for response with timeout
                let ping_future = pinger.ping(PingSequence(seq), &payload);
                let result = tokio::time::timeout(Duration::from_secs(2), ping_future).await;

                let latency = match result {
                    Ok(Ok((_, duration))) => Some(duration),
                    Ok(Err(_)) | Err(_) => None,
                };

                // Update statistics
                {
                    let mut stats_lock = stats.write();
                    if let Some(Stats::Ping(host_stats)) = stats_lock.get_mut(&ip) {
                        host_stats.update(latency);
                    }
                }

                seq = seq.wrapping_add(1);
            }
            _ = shutdown.changed() => {
                // Graceful shutdown
                break;
            }
        }
    }
}
