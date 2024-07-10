use {
    crate::{Timestamp, Uint64},
    serde::{Deserialize, Serialize},
};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename = "snake_case")]

pub enum Expiration {
    AtHeight(Uint64),
    AtTime(Timestamp),
}

impl Expiration {
    pub fn new_height(height: u64) -> Self {
        Self::AtHeight(Uint64::new(height))
    }

    pub fn new_time(timestamp: Timestamp) -> Self {
        Self::AtTime(timestamp)
    }
}
