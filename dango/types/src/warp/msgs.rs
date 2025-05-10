use {
    crate::gateway::bridge::BridgeMsg,
    grug::Addr,
    hyperlane_types::recipients::{RecipientMsg, RecipientQuery, RecipientQueryResponse},
};

#[grug::derive(Serde)]
pub struct InstantiateMsg {
    /// Address of the mailbox contract.
    pub mailbox: Addr,
}

#[grug::derive(Serde)]
pub enum ExecuteMsg {
    /// Required Hyperlane recipient interface.
    Recipient(RecipientMsg),
    /// Required Dango Gateway interface.
    Bridge(BridgeMsg),
}

#[grug::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    /// Query the address of the mailbox contract.
    #[returns(Addr)]
    Mailbox {},
    /// Required Hyperlane recipient interface.
    #[returns(RecipientQueryResponse)]
    Recipient(RecipientQuery),
}
