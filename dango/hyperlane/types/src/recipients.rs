use {
    crate::{Addr32, mailbox::Domain},
    grug_types::{Addr, HexBinary},
};

#[grug_types::derive(Serde)]
pub enum ExecuteMsg {
    Recipient(RecipientMsg),
}

#[grug_types::derive(Serde)]
pub enum RecipientMsg {
    Handle {
        origin_domain: Domain,
        sender: Addr32,
        body: HexBinary,
    },
}

#[grug_types::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    #[returns(RecipientQueryResponse)]
    Recipient(RecipientQuery),
}

#[grug_types::derive(Serde)]
pub enum RecipientQuery {
    /// Return the ISM this recipient would like to use for verifying incoming
    /// messages.
    /// `None` if the recipient would like to defer to the default ISM.
    InterchainSecurityModule {},
}

#[grug_types::derive(Serde)]
pub enum RecipientQueryResponse {
    InterchainSecurityModule(Option<Addr>),
}

impl RecipientQueryResponse {
    pub fn into_interchain_security_module(self) -> Option<Addr> {
        match self {
            RecipientQueryResponse::InterchainSecurityModule(ism) => ism,
        }
    }
}
