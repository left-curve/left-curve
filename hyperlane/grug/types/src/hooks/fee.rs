use {
    crate::hooks::{HookMsg, HookQuery, HookQueryResponse},
    grug::Addr,
};

// --------------------------------- messages ----------------------------------

#[grug::derive(Serde)]
pub struct InstantiateMsg {
    /// Address of the mailbox contract.
    pub mailbox: Addr,
}

#[grug::derive(Serde)]
pub enum ExecuteMsg {
    /// Required Hyperlane hook interface.
    Hook(HookMsg),
}

#[grug::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    /// Query the mailbox contract address.
    #[returns(Addr)]
    Mailbox {},
    /// Required Hyperlane hook interface.
    #[returns(HookQueryResponse)]
    Hook(HookQuery),
}
