use {
    crate::Addr32,
    grug::{Addr, HexBinary},
};

#[grug::derive(Serde)]
pub enum ExecuteMsg {
    Handle {
        origin: u32,
        sender: Addr32,
        body: HexBinary,
    },
}

#[grug::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    #[returns(Option<Addr>)]
    InterchainSecurityModule {},
}
