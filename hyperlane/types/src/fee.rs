use grug::{Addr, Coins, HexBinary};

// --------------------------------- messages ----------------------------------

#[grug::derive(Serde)]
pub struct InstantiateMsg {
    /// Address of the mailbox contract.
    pub mailbox: Addr,
}

#[grug::derive(Serde)]
pub enum ExecuteMsg {
    // Required Hyperlane hook interface.
    PostDispatch {
        raw_message: HexBinary,
        metadata: HexBinary,
    },
}

#[grug::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    /// Query the mailbox contract address.
    #[returns(Addr)]
    Mailbox {},
    // Required Hyperlane hook interface.
    #[returns(Coins)]
    QuoteDispatch {
        raw_message: HexBinary,
        metadata: HexBinary,
    },
}
