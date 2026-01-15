use crate::pinger;
use crate::stats::PingStats;
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
    pub hosts: Vec<Host>,
    pub selected_index: usize,
    pub ping_stats: Arc<RwLock<HashMap<IpAddr, PingStats>>>,
    pub should_quit: bool,
    ping_handles: HashMap<IpAddr, tokio::task::JoinHandle<()>>,
    shutdown_senders: HashMap<IpAddr, watch::Sender<bool>>,
}

impl App {
    pub fn new(ips: Vec<IpAddr>) -> Self {
        let hosts: Vec<Host> = ips
            .into_iter()
            .map(|ip| Host {
                ip,
                selected: false,
            })
            .collect();

        let ping_stats = Arc::new(RwLock::new(HashMap::new()));

        // Initialize stats for all hosts
        {
            let mut stats = ping_stats.write();
            for host in &hosts {
                stats.insert(host.ip, PingStats::new());
            }
        }

        Self {
            hosts,
            selected_index: 0,
            ping_stats,
            should_quit: false,
            ping_handles: HashMap::new(),
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

        if !selected {
            self.start_ping_task(ip);
        } else {
            self.stop_ping_task(ip);
        }
    }

    fn start_ping_task(&mut self, ip: IpAddr) {
        // Don't start if already running
        if self.ping_handles.contains_key(&ip) {
            return;
        }

        // Create shutdown channel
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        // Clone Arc for the async task
        let stats = Arc::clone(&self.ping_stats);

        // Spawn ping task
        let handle = tokio::spawn(async move {
            pinger::start_ping_task(ip, stats, shutdown_rx).await;
        });

        self.ping_handles.insert(ip, handle);
        self.shutdown_senders.insert(ip, shutdown_tx);
    }

    fn stop_ping_task(&mut self, ip: IpAddr) {
        // Send shutdown signal
        if let Some(sender) = self.shutdown_senders.remove(&ip) {
            let _ = sender.send(true);
        }

        // Abort the task
        if let Some(handle) = self.ping_handles.remove(&ip) {
            handle.abort();
        }
    }

    pub async fn shutdown(&mut self) {
        // Stop all ping tasks
        let ips: Vec<IpAddr> = self.ping_handles.keys().copied().collect();
        for ip in ips {
            self.stop_ping_task(ip);
        }

        // Give tasks a moment to clean up
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
}
