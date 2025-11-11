use {
    crate::{Block, BlockOutcome, Hash256},
    borsh::{BorshDeserialize, BorshSerialize},
    serde::{Deserialize, Serialize},
    std::{
        collections::HashMap,
        time::{SystemTime, UNIX_EPOCH},
    },
};

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, Eq, PartialEq)]
pub struct HttpRequestDetails {
    pub remote_ip: Option<String>,
    pub peer_ip: Option<String>,
    // For when I need to clean up old requests
    // Unix timestamp because I can't borsh serialize a DateTime
    pub created_at: u64,
}

impl HttpRequestDetails {
    pub fn new(remote_ip: Option<String>, peer_ip: Option<String>) -> Self {
        Self {
            remote_ip,
            peer_ip,
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }
}

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, Eq, PartialEq)]
pub struct BlockAndBlockOutcomeWithHttpDetails {
    pub block: Block,
    pub block_outcome: BlockOutcome,
    pub transactions_http_request_details: HashMap<Hash256, HttpRequestDetails>,
}
