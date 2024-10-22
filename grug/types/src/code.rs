use {
    crate::{Binary, StdError},
    borsh::{BorshDeserialize, BorshSerialize},
    serde::{Deserialize, Serialize},
};

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Code {
    pub code: Binary,
    pub status: CodeStatus,
}

#[derive(
    Serialize, Deserialize, BorshSerialize, BorshDeserialize, Clone, Copy, Debug, PartialEq, Eq,
)]
#[serde(rename_all = "snake_case")]
pub enum CodeStatus {
    Orphan { since: u64 },
    Usage { usage: u64 },
}

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Clone, Copy, Debug)]
#[serde(rename_all = "snake_case")]
pub enum CodeStatusType {
    Orphan,
    Usage,
}

impl TryFrom<u8> for CodeStatusType {
    type Error = StdError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(CodeStatusType::Orphan),
            1 => Ok(CodeStatusType::Usage),
            _ => Err(StdError::deserialize::<CodeStatusType, _>(
                "From<u8>",
                format!("Invalid u8: {value}"),
            )),
        }
    }
}
