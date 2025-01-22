use std::ops::Add;

use grug::{Addr, HexBinary, HexByteArray};

// ----------------------------------- types -----------------------------------

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
    pub local_domain: u32,
}

// --------------------------------- messages ----------------------------------

#[grug::derive(Serde)]
pub struct InstantiateMsg {
    pub mailbox: Addr,
}

#[grug::derive(Serde)]
pub enum ExecuteMsg {
    Announce {
        validator: HexBinary,
        signature: HexBinary,
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
pub struct Initialized {
    pub creator: Addr,
    pub mailbox: Addr,
    pub local_domain: u32,
}

#[grug::derive(Serde)]
pub struct Announcement {
    pub sender: Addr,
    pub validator: HexBinary,
    pub storage_location: String,
}
