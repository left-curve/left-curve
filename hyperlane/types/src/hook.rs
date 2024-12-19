use grug::{Coins, HexBinary};

#[grug::derive(Serde)]
pub enum ExecuteMsg {
    PostDispatch {
        raw_message: HexBinary,
        metadata: HexBinary,
    },
}

#[grug::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    #[returns(Coins)]
    QuoteDispatch {
        raw_message: HexBinary,
        metadata: HexBinary,
    },
}
