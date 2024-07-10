use {
    crate::{Timestamp, Uint64},
    serde::{Deserialize, Serialize},
    std::fmt::{Display, Formatter},
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

impl Display for Expiration {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            Expiration::AtHeight(height) => write!(f, "expiration height: {height}"),
            Expiration::AtTime(time) => write!(f, "expiration time: {} ns", time.nanos()),
        }
    }
}
