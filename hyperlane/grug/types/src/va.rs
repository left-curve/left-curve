use {
    grug::{Addr, HexByteArray},
    std::collections::{BTreeMap, BTreeSet},
};

pub const VA_DOMAIN_KEY: &str = "HYPERLANE_ANNOUNCEMENT";

// --------------------------------- messages ----------------------------------

#[grug::derive(Serde)]
pub struct InstantiateMsg {
    pub mailbox: Addr,
}

#[grug::derive(Serde)]
pub enum ExecuteMsg {
    Announce {
        validator: HexByteArray<20>,
        signature: HexByteArray<65>,
        storage_location: String,
    },
}

#[grug::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    /// Query the mailbox contract address.
    #[returns(Addr)]
    Mailbox {},
    /// Query the set of validators who have announced their storage locations.
    #[returns(BTreeSet<HexByteArray<20>>)]
    AnnouncedValidators {
        start_after: Option<HexByteArray<20>>,
        limit: Option<u32>,
    },
    /// Query the storage locations of the given validators.
    #[returns(BTreeMap<HexByteArray<20>, BTreeSet<String>>)]
    AnnouncedStorageLocations {
        validators: BTreeSet<HexByteArray<20>>,
    },
}

// ---------------------------------- events -----------------------------------

#[grug::derive(Serde)]
#[grug::event("init_validator_announce")]
pub struct Initialize {
    pub creator: Addr,
    pub mailbox: Addr,
}

#[grug::derive(Serde)]
#[grug::event("validator_announcement")]
pub struct Announce {
    pub sender: Addr,
    pub validator: HexByteArray<20>,
    pub storage_location: String,
}
