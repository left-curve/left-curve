use {
    borsh::{BorshDeserialize, BorshSerialize},
    serde::{Deserialize, Serialize},
};

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, Eq, PartialEq)]
pub struct HttpRequestDetails {
    pub remote_ip: Option<String>,
    pub peer_ip: Option<String>,
    // For when I need to clean up old requests
    // pub created_at: i64, // Unix timestamp
}

impl HttpRequestDetails {
    pub fn new(remote_ip: Option<String>, peer_ip: Option<String>) -> Self {
        Self {
            remote_ip,
            peer_ip,
            // created_at: chrono::Utc::now().timestamp(),
        }
    }
}
