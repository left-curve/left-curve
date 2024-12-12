use {
    crate::{Addr, Binary, Json},
    borsh::{BorshDeserialize, BorshSerialize},
    serde::{Deserialize, Serialize},
    std::collections::BTreeMap,
};

pub type IbcClientId = u32;

#[derive(
    Serialize,
    Deserialize,
    BorshSerialize,
    BorshDeserialize,
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
)]
pub enum IbcClientType {
    /// ICS-07 Tendermint client:
    /// <https://github.com/cosmos/ibc/tree/main/spec/client/ics-007-tendermint-client>
    #[serde(rename = "07-tendermint")]
    Tendermint,
    /// ICS-11 CometBLS client:
    /// <https://github.com/unionlabs/union/tree/main/11-cometbls>
    #[serde(rename = "11-cometbls")]
    CometBls,
}

impl IbcClientType {
    pub fn as_str(&self) -> &'static str {
        match self {
            IbcClientType::Tendermint => "07-tendermint",
            IbcClientType::CometBls => "11-cometbls",
        }
    }

    /// Return an iterator that enumerates all client types exhaustively.
    /// This is used for constructing `IbcClientImpls` instances.
    pub fn iter() -> impl Iterator<Item = IbcClientType> {
        [Self::Tendermint, Self::CometBls].into_iter()
    }
}

/// A mapping from client type to the contract address that implements the client.
///
/// This implementation guarantees the mapping is exhaustive, meaning no client
/// type is missing.
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Clone, Debug, PartialEq, Eq)]
pub struct IbcClientImpls(BTreeMap<IbcClientType, Addr>);

impl IbcClientImpls {
    pub fn new_unchecked(inner: BTreeMap<IbcClientType, Addr>) -> Self {
        Self(inner)
    }

    pub fn new<F>(getter: F) -> Self
    where
        F: Fn(IbcClientType) -> Addr,
    {
        Self(IbcClientType::iter().map(|ty| (ty, getter(ty))).collect())
    }

    pub fn get(&self, ty: IbcClientType) -> Addr {
        // This is safe because the mapping is guaranteed to be exhaustive.
        self.0[&ty]
    }
}

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Clone, Debug, PartialEq, Eq)]
pub enum IbcClientQuery {
    /// During client creation, verify the client and consensus states are valid.
    ///
    /// Return the latest consensus height, as well as client and consensus
    /// states encoded as raw bytes with the appropriate encoding scheme.
    VerifyCreation {
        client_state: Json,
        consensus_state: Json,
    },
}

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Clone, Debug, PartialEq, Eq)]
pub enum IbcClientQueryResponse {
    VerifyCreation {
        latest_height: u64,
        raw_client_state: Binary,
        raw_consensus_state: Binary,
    },
}

impl IbcClientQueryResponse {
    pub fn as_verify_creation(self) -> (u64, Binary, Binary) {
        match self {
            IbcClientQueryResponse::VerifyCreation {
                latest_height,
                raw_client_state,
                raw_consensus_state,
            } => (latest_height, raw_client_state, raw_consensus_state),
            // _ => panic!("IbcClientQueryResponse is not VerifyCreation"),
        }
    }
}
