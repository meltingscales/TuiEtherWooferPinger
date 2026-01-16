use crate::http_stats::HttpStats;
use chrono::{DateTime, Local};
use std::collections::VecDeque;
use std::time::Duration;

const MAX_SAMPLES: usize = 100;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AppMode {
    Icmp,
    Http,
}

#[derive(Clone, Debug)]
pub enum Stats {
    Ping(PingStats),
    Http(HttpStats),
}

#[derive(Clone, Debug, PartialEq)]
pub enum PingStatus {
    NotStarted,
    Active,
    Timeout,
    Unreachable,
}

#[derive(Clone, Debug)]
pub struct PingStats {
    pub status: PingStatus,
    pub last_latency: Option<Duration>,
    pub avg_latency: Option<Duration>,
    pub min_latency: Option<Duration>,
    pub max_latency: Option<Duration>,
    pub packets_sent: u64,
    pub packets_received: u64,
    pub packet_loss_percent: f64,
    pub last_updated: DateTime<Local>,
    recent_latencies: VecDeque<Duration>,
    consecutive_timeouts: u32,
}

impl Default for PingStats {
    fn default() -> Self {
        Self {
            status: PingStatus::NotStarted,
            last_latency: None,
            avg_latency: None,
            min_latency: None,
            max_latency: None,
            packets_sent: 0,
            packets_received: 0,
            packet_loss_percent: 0.0,
            last_updated: Local::now(),
            recent_latencies: VecDeque::with_capacity(MAX_SAMPLES),
            consecutive_timeouts: 0,
        }
    }
}

impl PingStats {
    pub fn new() -> Self {
        Self::default()
    }

    /// Update statistics with a new ping result
    pub fn update(&mut self, result: Option<Duration>) {
        self.packets_sent += 1;
        self.last_updated = Local::now();

        match result {
            Some(latency) => {
                self.packets_received += 1;
                self.last_latency = Some(latency);
                self.consecutive_timeouts = 0;
                self.status = PingStatus::Active;

                // Add to recent latencies (ring buffer)
                if self.recent_latencies.len() >= MAX_SAMPLES {
                    self.recent_latencies.pop_front();
                }
                self.recent_latencies.push_back(latency);

                // Calculate statistics
                self.calculate_stats();
            }
            None => {
                self.last_latency = None;
                self.consecutive_timeouts += 1;

                // Determine status based on consecutive timeouts
                if self.consecutive_timeouts >= 5 {
                    self.status = PingStatus::Unreachable;
                } else {
                    self.status = PingStatus::Timeout;
                }
            }
        }

        // Calculate packet loss
        if self.packets_sent > 0 {
            self.packet_loss_percent =
                ((self.packets_sent - self.packets_received) as f64 / self.packets_sent as f64)
                    * 100.0;
        }
    }

    fn calculate_stats(&mut self) {
        if self.recent_latencies.is_empty() {
            return;
        }

        // Calculate average
        let sum: Duration = self.recent_latencies.iter().sum();
        self.avg_latency = Some(sum / self.recent_latencies.len() as u32);

        // Calculate min/max
        self.min_latency = self.recent_latencies.iter().min().copied();
        self.max_latency = self.recent_latencies.iter().max().copied();
    }
}
