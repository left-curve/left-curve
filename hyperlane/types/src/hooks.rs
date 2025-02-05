pub mod fee;
pub mod merkle;

use grug::{Coins, HexBinary};

#[grug::derive(Serde)]
pub enum ExecuteMsg {
    Hook(HookMsg),
}

#[grug::derive(Serde)]
pub enum HookMsg {
    PostDispatch {
        raw_message: HexBinary,
        raw_metadata: HexBinary,
    },
}

#[grug::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    #[returns(HookQueryResponse)]
    Hook(HookQuery),
}

#[grug::derive(Serde)]
pub enum HookQuery {
    QuoteDispatch {
        raw_message: HexBinary,
        raw_metadata: HexBinary,
    },
}

#[grug::derive(Serde)]
pub enum HookQueryResponse {
    QuoteDispatch(Coins),
}

impl HookQueryResponse {
    pub fn as_quote_dispatch(self) -> Coins {
        match self {
            HookQueryResponse::QuoteDispatch(coins) => coins,
        }
    }
}
