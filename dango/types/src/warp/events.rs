use {
    grug::{Addr, Denom, HexBinary, Uint128},
    hyperlane_types::{Addr32, mailbox::Domain},
};

#[grug::derive(Serde)]
#[grug::event("transfer_remote")]
pub struct TransferRemote {
    pub sender: Addr,
    pub destination_domain: Domain,
    pub recipient: Addr32,
    pub token: Denom,
    pub amount: Uint128,
    pub hook: Option<Addr>,
    pub metadata: Option<HexBinary>,
}

#[grug::derive(Serde)]
#[grug::event("handle")]
pub struct Handle {
    pub recipient: Addr32,
    pub token: Denom,
    pub amount: Uint128,
}
