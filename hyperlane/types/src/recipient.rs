use {
    crate::{mailbox::Domain, Addr32},
    grug::{Addr, HexBinary},
};

#[grug::derive(Serde)]
pub enum ExecuteMsg {
    Handle {
        origin: Domain,
        sender: Addr32,
        body: HexBinary,
    },
}

#[grug::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    #[returns(Option<Addr>)]
    InterchainSecurityModule {},
}
