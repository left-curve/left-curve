use {
    crate::{token_minter::HookTransferRemote, warp::Route},
    grug::{Addr, Denom},
    hyperlane_types::{
        mailbox::Domain,
        recipients::{RecipientMsg, RecipientQuery, RecipientQueryResponse},
    },
};

#[grug::derive(Serde)]
pub struct InstantiateMsg {
    /// Address of the mailbox contract.
    pub mailbox: Addr,
}

#[grug::derive(Serde)]
pub enum ExecuteMsg {
    /// Define the recipient contract and withdrawal fee rate for a token on a
    /// destination domain.
    SetRoute {
        denom: Denom,
        destination_domain: Domain,
        route: Route,
    },
    /// Required Hyperlane recipient interface.
    Recipient(RecipientMsg),
    HookTransferRemote(HookTransferRemote),
}

#[grug::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    /// Query the address of the mailbox contract.
    #[returns(Addr)]
    Mailbox {},
    /// Query the recipient contract for a token on a destination domain.
    #[returns(Route)]
    Route {
        denom: Denom,
        destination_domain: Domain,
    },
    /// Enumerate all routes.
    #[returns(Vec<QueryRoutesResponseItem>)]
    Routes {
        start_after: Option<QueryRoutesPageParam>,
        limit: Option<u32>,
    },
    /// Required Hyperlane recipient interface.
    #[returns(RecipientQueryResponse)]
    Recipient(RecipientQuery),
}

#[grug::derive(Serde)]
pub struct QueryRoutesPageParam {
    pub denom: Denom,
    pub destination_domain: Domain,
}

#[grug::derive(Serde)]
pub struct QueryRoutesResponseItem {
    pub denom: Denom,
    pub destination_domain: Domain,
    pub route: Route,
}
