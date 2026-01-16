use chrono::{DateTime, Local};
use std::collections::VecDeque;
use std::time::Duration;

const MAX_SAMPLES: usize = 100;

#[derive(Clone, Debug, PartialEq)]
pub enum HttpStatus {
    NotStarted,
    Success,       // 2xx responses
    ClientError,   // 4xx responses
    ServerError,   // 5xx responses
    NetworkError,  // Connection/timeout errors
}

#[derive(Clone, Debug)]
pub struct HttpStats {
    pub status: HttpStatus,
    pub last_response_time: Option<Duration>,
    pub avg_response_time: Option<Duration>,
    pub min_response_time: Option<Duration>,
    pub max_response_time: Option<Duration>,
    pub last_status_code: Option<u16>,
    pub last_content_size: Option<u64>,
    pub last_error: Option<String>,
    pub requests_sent: u64,
    pub requests_successful: u64,
    pub success_rate_percent: f64,
    pub last_updated: DateTime<Local>,
    recent_times: VecDeque<Duration>,
}

impl Default for HttpStats {
    fn default() -> Self {
        Self {
            status: HttpStatus::NotStarted,
            last_response_time: None,
            avg_response_time: None,
            min_response_time: None,
            max_response_time: None,
            last_status_code: None,
            last_content_size: None,
            last_error: None,
            requests_sent: 0,
            requests_successful: 0,
            success_rate_percent: 0.0,
            last_updated: Local::now(),
            recent_times: VecDeque::with_capacity(MAX_SAMPLES),
        }
    }
}

impl HttpStats {
    pub fn new() -> Self {
        Self::default()
    }

    /// Update statistics with a new HTTP result
    pub fn update(
        &mut self,
        status_code: Option<u16>,
        response_time: Duration,
        content_size: Option<u64>,
        error: Option<String>,
    ) {
        self.requests_sent += 1;
        self.last_updated = Local::now();
        self.last_response_time = Some(response_time);
        self.last_content_size = content_size;
        self.last_error = error.clone();

        match status_code {
            Some(code) => {
                self.last_status_code = Some(code);
                self.requests_successful += 1;

                // Determine status based on code
                self.status = match code {
                    200..=299 => HttpStatus::Success,
                    400..=499 => HttpStatus::ClientError,
                    500..=599 => HttpStatus::ServerError,
                    _ => HttpStatus::Success, // 1xx, 3xx treated as success
                };

                // Add to recent times (ring buffer)
                if self.recent_times.len() >= MAX_SAMPLES {
                    self.recent_times.pop_front();
                }
                self.recent_times.push_back(response_time);

                // Calculate statistics
                self.calculate_stats();
            }
            None => {
                // Network error
                self.status = HttpStatus::NetworkError;
                self.last_status_code = None;
            }
        }

        // Calculate success rate
        if self.requests_sent > 0 {
            self.success_rate_percent =
                (self.requests_successful as f64 / self.requests_sent as f64) * 100.0;
        }
    }

    fn calculate_stats(&mut self) {
        if self.recent_times.is_empty() {
            return;
        }

        // Calculate average
        let sum: Duration = self.recent_times.iter().sum();
        self.avg_response_time = Some(sum / self.recent_times.len() as u32);

        // Calculate min/max
        self.min_response_time = self.recent_times.iter().min().copied();
        self.max_response_time = self.recent_times.iter().max().copied();
    }
}
