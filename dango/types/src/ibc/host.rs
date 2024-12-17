use {
    crate::ibc::client::Height,
    grug::{Addr, Binary, Json, PrimaryKey, StdError, StdResult},
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
    /// Associate a client implementation with a client type.
    RegisterClient {
        client_type: ClientType,
        client_impl: Addr,
    },
    /// Create an instance of a client of the given type.
    CreateClient {
        client_type: ClientType,
        client_state: Json,
        consensus_state: Json,
    },
    /// Update the state of a client.
    UpdateClient {
        client_id: ClientId,
        client_message: Json,
    },
}

#[grug::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    /// Query the implementation contract of a given client type.
    #[returns(Addr)]
    ClientImpl { client_type: ClientType },
    /// Enumerate implementation contracts for all client types.
    #[returns(BTreeMap<ClientType, Addr>)]
    ClientImpls {
        start_after: Option<ClientType>,
        limit: Option<u32>,
    },
    /// Query a single client by ID.
    #[returns(Client)]
    Client { client_id: ClientId },
    /// Enumerate all clients.
    #[returns(BTreeMap<ClientId, Client>)]
    Clients {
        start_after: Option<ClientId>,
        limit: Option<u32>,
    },
    /// Query the state of a client.
    #[returns(Binary)]
    ClientState { client_id: ClientId },
    /// Enumerate states of all clients.
    #[returns(BTreeMap<ClientId, Binary>)]
    ClientStates {
        start_after: Option<ClientId>,
        limit: Option<u32>,
    },
    /// Query the consensus state of a client at a given height.
    #[returns(Binary)]
    ConsensusState { client_id: ClientId, height: Height },
    /// Enumerate consensus states of a client of all heights.
    #[returns(BTreeMap<Height, Binary>)]
    ConsensusStates {
        client_id: ClientId,
        start_after: Option<Height>,
        limit: Option<u32>,
    },
}
