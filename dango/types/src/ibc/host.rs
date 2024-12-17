use {
    grug::{Addr, Json, PrimaryKey, StdError, StdResult},
    std::{borrow::Cow, collections::BTreeMap},
};

pub type ClientId = u32;

// In ibc-union, the key and value of a commitment are both Keccak256 hashes.
pub type Commitment = [u8; 32];

#[grug::derive(Serde, Borsh)]
#[derive(Copy, PartialOrd, Ord)]
pub enum ClientType {
    #[serde(rename = "07-tendermint")]
    Tendermint,
    #[serde(rename = "11-cometbls")]
    CometBls,
}

impl PrimaryKey for ClientType {
    type Output = Self;
    type Prefix = ();
    type Suffix = ();

    const KEY_ELEMS: u8 = 1;

    fn raw_keys(&self) -> Vec<Cow<[u8]>> {
        let bytes = match self {
            ClientType::Tendermint => &[7],
            ClientType::CometBls => &[11],
        };

        vec![Cow::Borrowed(bytes)]
    }

    fn from_slice(bytes: &[u8]) -> StdResult<Self::Output> {
        match bytes {
            [7] => Ok(ClientType::Tendermint),
            [11] => Ok(ClientType::CometBls),
            _ => Err(StdError::deserialize::<Self::Output, _>(
                "key",
                format!("invalid client type: {bytes:?}! must be 7|11"),
            )),
        }
    }
}

#[grug::derive(Serde, Borsh)]
pub struct Client {
    pub client_type: ClientType,
    pub client_impl: Addr,
}

#[grug::derive(Serde)]
pub struct InstantiateMsg {
    pub client_impls: BTreeMap<ClientType, Addr>,
}

#[grug::derive(Serde)]
pub enum ExecuteMsg {
    RegisterClients(BTreeMap<ClientType, Addr>),
    CreateClient {
        client_type: ClientType,
        client_state: Json,
        consensus_state: Json,
    },
}

#[grug::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    #[returns(Addr)]
    ClientImpl(ClientType),
    #[returns(BTreeMap<ClientType, Addr>)]
    ClientImpls {
        start_after: Option<ClientType>,
        limit: Option<u32>,
    },
}
