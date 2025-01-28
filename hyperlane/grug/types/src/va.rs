use {
    crate::mailbox::Domain,
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
    #[returns(BTreeMap<HexByteArray<20>, BTreeSet<String>>)]
    AnnounceStorageLocations {
        validators: BTreeSet<HexByteArray<20>>,
    },

    #[returns(BTreeSet<HexByteArray<20>>)]
    AnnouncedValidators {},

    #[returns(Addr)]
    Mailbox {},

    #[returns(Domain)]
    LocalDomain {},
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
