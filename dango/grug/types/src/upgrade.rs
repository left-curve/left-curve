use {
    borsh::{BorshDeserialize, BorshSerialize},
    serde::{Deserialize, Serialize},
    serde_with::skip_serializing_none,
};

/// An upgrade planned for a future block.
#[skip_serializing_none]
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct NextUpgrade {
    /// A block height in the future at which this upgrade is planned to happen.
    pub height: u64,
    pub cargo_version: String,
    pub git_tag: Option<String>,
    /// URL that links to additional information describing this upgrade.
    pub url: Option<String>,
}

#[skip_serializing_none]
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct PastUpgrade {
    pub cargo_version: String,
    pub git_tag: Option<String>,
    pub url: Option<String>,
}

impl From<NextUpgrade> for PastUpgrade {
    fn from(upgrade: NextUpgrade) -> Self {
        Self {
            cargo_version: upgrade.cargo_version,
            git_tag: upgrade.git_tag,
            url: upgrade.url,
        }
    }
}
