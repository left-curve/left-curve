use grug::{Addr, HexBinary, HexByteArray};

use crate::mailbox::Domain;

// ----------------------------------- types -----------------------------------

pub const VA_DOMAIN_KEY: &str = "HYPERLANE_ANNOUNCEMENT";

#[grug::derive(Serde)]
pub struct GetAnnounceStorageLocationsResponse {
    pub storage_locations: Vec<(String, Vec<String>)>,
}

#[grug::derive(Serde)]
pub struct GetAnnouncedValidatorsResponse {
    pub validators: Vec<HexByteArray<20>>,
}

#[grug::derive(Serde)]
pub struct MailboxResponse {
    pub mailbox: String,
}

#[grug::derive(Serde)]
pub struct LocalDomainResponse {
    pub local_domain: Domain,
}

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
    #[returns(GetAnnounceStorageLocationsResponse)]
    GetAnnounceStorageLocations { validators: Vec<HexBinary> },

    #[returns(GetAnnouncedValidatorsResponse)]
    GetAnnouncedValidators {},

    #[returns(MailboxResponse)]
    Mailbox {},

    #[returns(LocalDomainResponse)]
    LocalDomain {},
}

// ---------------------------------- events -----------------------------------

#[grug::derive(Serde)]
pub struct EvtInitialize {
    pub creator: Addr,
    pub mailbox: Addr,
    pub local_domain: Domain,
}

#[grug::derive(Serde)]
pub struct EvtAnnouncement {
    pub sender: Addr,
    pub validator: HexByteArray<20>,
    pub storage_location: String,
}
