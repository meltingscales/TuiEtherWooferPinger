use crate::http_checker;
use crate::http_stats::HttpStats;
use crate::pinger;
use crate::stats::{AppMode, PingStats, Stats};
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use parking_lot::RwLock;
use std::collections::HashMap;
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
