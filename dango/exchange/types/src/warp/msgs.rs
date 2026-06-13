use {
    crate::gateway::bridge::BridgeMsg,
    dango_hyperlane_types::recipients::{RecipientMsg, RecipientQuery, RecipientQueryResponse},
    dango_primitives::Addr,
};

#[dango_primitives::derive(Serde)]
pub struct InstantiateMsg {
    /// Address of the mailbox contract.
    pub mailbox: Addr,
}

#[dango_primitives::derive(Serde)]
pub enum ExecuteMsg {
    /// Required Hyperlane recipient interface.
    Recipient(RecipientMsg),
    /// Required Dango Gateway interface.
    Bridge(BridgeMsg),
}

#[dango_primitives::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    /// Query the address of the mailbox contract.
    #[returns(Addr)]
    Mailbox {},
    /// Required Hyperlane recipient interface.
    #[returns(RecipientQueryResponse)]
    Recipient(RecipientQuery),
}
