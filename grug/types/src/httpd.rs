use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpdConfig {
    pub enabled: bool,
    pub ip: String,
    pub port: u16,
    pub cors_allowed_origin: Option<String>,
    pub static_files_path: Option<String>,
    pub workers: usize,
    pub max_connections: usize,
    pub backlog: u32,
    pub keep_alive_secs: u64,
    pub client_request_timeout_secs: u64,
    pub client_disconnect_timeout_secs: u64,
    pub worker_max_blocking_threads: usize,
    pub max_subscriptions_per_connection: usize,
    pub max_subscriptions_global: usize,
}

impl Default for HttpdConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            ip: "127.0.0.1".to_string(),
            port: 8080,
            cors_allowed_origin: None,
            static_files_path: None,
            workers: 12,
            max_connections: 10_000,
            backlog: 2048,
            keep_alive_secs: 5,
            client_request_timeout_secs: 2,
            client_disconnect_timeout_secs: 1,
            worker_max_blocking_threads: 8,
            max_subscriptions_per_connection: 25,
            max_subscriptions_global: 5000,
        }
    }
}
