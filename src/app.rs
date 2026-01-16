use crate::http_checker;
use crate::http_stats::HttpStats;
use crate::pinger;
use crate::stats::{AppMode, PingStats, Stats};
use anyhow::Result;
use chrono::Local;
use crossterm::event::{KeyCode, KeyEvent};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::net::IpAddr;
use std::sync::Arc;
use tokio::sync::watch;

#[derive(Clone, Debug)]
pub struct Host {
    pub ip: IpAddr,
    pub selected: bool,
}

pub struct App {
    pub mode: AppMode,
    pub port: u16,
    pub hosts: Vec<Host>,
    pub selected_index: usize,
    pub stats: Arc<RwLock<HashMap<IpAddr, Stats>>>,
    pub should_quit: bool,
    pub paused: bool,
    task_handles: HashMap<IpAddr, tokio::task::JoinHandle<()>>,
    shutdown_senders: HashMap<IpAddr, watch::Sender<bool>>,
}

impl App {
    pub fn new(ips: Vec<IpAddr>, mode: AppMode, port: u16) -> Self {
        let hosts: Vec<Host> = ips
            .into_iter()
            .map(|ip| Host {
                ip,
                selected: false,
            })
            .collect();

        let stats_map = Arc::new(RwLock::new(HashMap::new()));

        // Initialize stats for all hosts based on mode
        {
            let mut stats = stats_map.write();
            for host in &hosts {
                let stat = match mode {
                    AppMode::Icmp => Stats::Ping(PingStats::new()),
                    AppMode::Http => Stats::Http(HttpStats::new()),
                };
                stats.insert(host.ip, stat);
            }
        }

        Self {
            mode,
            port,
            hosts,
            selected_index: 0,
            stats: stats_map,
            should_quit: false,
            paused: false,
            task_handles: HashMap::new(),
            shutdown_senders: HashMap::new(),
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.should_quit = true;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.move_selection(-1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.move_selection(1);
            }
            KeyCode::Char(' ') => {
                self.toggle_selection();
            }
            KeyCode::Char('p') => {
                self.toggle_pause();
            }
            KeyCode::Char('a') => {
                self.select_all();
            }
            KeyCode::Char('d') => {
                self.deselect_all();
            }
            KeyCode::Char('s') => {
                self.export_stats()?;
            }
            _ => {}
        }
        Ok(())
    }

    fn move_selection(&mut self, delta: isize) {
        if self.hosts.is_empty() {
            return;
        }

        let len = self.hosts.len() as isize;
        let new_index = (self.selected_index as isize + delta + len) % len;
        self.selected_index = new_index as usize;
    }

    fn toggle_selection(&mut self) {
        if self.hosts.is_empty() {
            return;
        }

        let ip = self.hosts[self.selected_index].ip;
        let selected = self.hosts[self.selected_index].selected;

        self.hosts[self.selected_index].selected = !selected;

        // Only start/stop tasks if not paused
        if !self.paused {
            if !selected {
                self.start_task(ip);
            } else {
                self.stop_task(ip);
            }
        }
    }

    fn toggle_pause(&mut self) {
        self.paused = !self.paused;

        if self.paused {
            // Stop all running tasks
            let ips: Vec<IpAddr> = self.task_handles.keys().copied().collect();
            for ip in ips {
                self.stop_task(ip);
            }
        } else {
            // Restart tasks for all selected hosts
            let selected_ips: Vec<IpAddr> = self
                .hosts
                .iter()
                .filter(|h| h.selected)
                .map(|h| h.ip)
                .collect();

            for ip in selected_ips {
                self.start_task(ip);
            }
        }
    }

    fn start_task(&mut self, ip: IpAddr) {
        // Don't start if already running
        if self.task_handles.contains_key(&ip) {
            return;
        }

        // Create shutdown channel
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        // Clone Arc for the async task
        let stats = Arc::clone(&self.stats);
        let port = self.port;

        // Spawn task based on mode
        let handle = match self.mode {
            AppMode::Icmp => tokio::spawn(async move {
                pinger::start_ping_task(ip, stats, shutdown_rx).await;
            }),
            AppMode::Http => tokio::spawn(async move {
                http_checker::start_http_task(ip, port, stats, shutdown_rx).await;
            }),
        };

        self.task_handles.insert(ip, handle);
        self.shutdown_senders.insert(ip, shutdown_tx);
    }

    fn stop_task(&mut self, ip: IpAddr) {
        // Send shutdown signal
        if let Some(sender) = self.shutdown_senders.remove(&ip) {
            let _ = sender.send(true);
        }

        // Abort the task
        if let Some(handle) = self.task_handles.remove(&ip) {
            handle.abort();
        }
    }

    fn select_all(&mut self) {
        if self.paused {
            // Just mark as selected, don't start tasks
            for host in &mut self.hosts {
                host.selected = true;
            }
        } else {
            // Mark as selected and start tasks
            for i in 0..self.hosts.len() {
                let ip = self.hosts[i].ip;
                if !self.hosts[i].selected {
                    self.hosts[i].selected = true;
                    self.start_task(ip);
                }
            }
        }
    }

    fn deselect_all(&mut self) {
        // Stop all tasks and mark as not selected
        let ips: Vec<IpAddr> = self.hosts.iter().map(|h| h.ip).collect();
        for ip in ips {
            self.stop_task(ip);
        }

        for host in &mut self.hosts {
            host.selected = false;
        }
    }

    fn export_stats(&self) -> Result<()> {
        let timestamp = Local::now().format("%Y%m%d_%H%M%S");
        let filename = format!("stats_export_{}.csv", timestamp);

        let mut file = File::create(&filename)?;

        // Write header based on mode
        match self.mode {
            AppMode::Icmp => {
                writeln!(
                    file,
                    "IP,Status,Last Latency (ms),Avg Latency (ms),Min Latency (ms),Max Latency (ms),Packet Loss %,Packets Sent,Packets Received"
                )?;

                let stats_lock = self.stats.read();
                for host in &self.hosts {
                    if let Some(Stats::Ping(stats)) = stats_lock.get(&host.ip) {
                        writeln!(
                            file,
                            "{},{:?},{},{},{},{},{:.2},{},{}",
                            host.ip,
                            stats.status,
                            stats
                                .last_latency
                                .map(|d| format!("{:.2}", d.as_secs_f64() * 1000.0))
                                .unwrap_or_else(|| "-".to_string()),
                            stats
                                .avg_latency
                                .map(|d| format!("{:.2}", d.as_secs_f64() * 1000.0))
                                .unwrap_or_else(|| "-".to_string()),
                            stats
                                .min_latency
                                .map(|d| format!("{:.2}", d.as_secs_f64() * 1000.0))
                                .unwrap_or_else(|| "-".to_string()),
                            stats
                                .max_latency
                                .map(|d| format!("{:.2}", d.as_secs_f64() * 1000.0))
                                .unwrap_or_else(|| "-".to_string()),
                            stats.packet_loss_percent,
                            stats.packets_sent,
                            stats.packets_received
                        )?;
                    }
                }
            }
            AppMode::Http => {
                writeln!(
                    file,
                    "IP,Status,Status Code,Last Response Time (ms),Avg Response Time (ms),Min Response Time (ms),Max Response Time (ms),Content Size,Success Rate %,Requests Sent,Requests Successful,Last Error"
                )?;

                let stats_lock = self.stats.read();
                for host in &self.hosts {
                    if let Some(Stats::Http(stats)) = stats_lock.get(&host.ip) {
                        writeln!(
                            file,
                            "{},{:?},{},{},{},{},{},{},{:.2},{},{},{}",
                            host.ip,
                            stats.status,
                            stats
                                .last_status_code
                                .map(|c| c.to_string())
                                .unwrap_or_else(|| "-".to_string()),
                            stats
                                .last_response_time
                                .map(|d| format!("{:.2}", d.as_secs_f64() * 1000.0))
                                .unwrap_or_else(|| "-".to_string()),
                            stats
                                .avg_response_time
                                .map(|d| format!("{:.2}", d.as_secs_f64() * 1000.0))
                                .unwrap_or_else(|| "-".to_string()),
                            stats
                                .min_response_time
                                .map(|d| format!("{:.2}", d.as_secs_f64() * 1000.0))
                                .unwrap_or_else(|| "-".to_string()),
                            stats
                                .max_response_time
                                .map(|d| format!("{:.2}", d.as_secs_f64() * 1000.0))
                                .unwrap_or_else(|| "-".to_string()),
                            stats
                                .last_content_size
                                .map(|s| s.to_string())
                                .unwrap_or_else(|| "-".to_string()),
                            stats.success_rate_percent,
                            stats.requests_sent,
                            stats.requests_successful,
                            stats
                                .last_error
                                .as_ref()
                                .map(|e| format!("\"{}\"", e.replace('"', "'")))
                                .unwrap_or_else(|| "-".to_string())
                        )?;
                    }
                }
            }
        }

        // Write success - we can't show a message in the TUI easily, but the file is created
        Ok(())
    }

    pub async fn shutdown(&mut self) {
        // Stop all tasks
        let ips: Vec<IpAddr> = self.task_handles.keys().copied().collect();
        for ip in ips {
            self.stop_task(ip);
        }

        // Give tasks a moment to clean up
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
}
