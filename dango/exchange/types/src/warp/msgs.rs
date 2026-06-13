use {
    crate::gateway::bridge::BridgeMsg,
    grug_types::Addr,
    hyperlane_types::recipients::{RecipientMsg, RecipientQuery, RecipientQueryResponse},
};

#[grug_types::derive(Serde)]
pub struct InstantiateMsg {
    /// Address of the mailbox contract.
    pub mailbox: Addr,
}

#[grug_types::derive(Serde)]
pub enum ExecuteMsg {
    /// Required Hyperlane recipient interface.
    Recipient(RecipientMsg),
    /// Required Dango Gateway interface.
    Bridge(BridgeMsg),
}

#[grug_types::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    /// Query the address of the mailbox contract.
    #[returns(Addr)]
    Mailbox {},
    /// Required Hyperlane recipient interface.
    #[returns(RecipientQueryResponse)]
    Recipient(RecipientQuery),
}
