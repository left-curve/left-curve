use {
    crate::warp::{RateLimit, Route},
    grug::{Addr, Denom, HexBinary, Uint128},
    hyperlane_types::{
        Addr32,
        mailbox::Domain,
        recipients::{RecipientMsg, RecipientQuery, RecipientQueryResponse},
    },
    std::collections::BTreeMap,
};

#[grug::derive(Serde)]
pub struct InstantiateMsg {
    /// Address of the mailbox contract.
    pub mailbox: Addr,
}

#[grug::derive(Serde)]
pub enum ExecuteMsg {
    /// Sends tokens to a remote domain.
    ///
    /// Sender must attach exactly one token that is greater than the withdrawal
    /// fee.
    ///
    /// ## Notes:
    ///
    /// We currently don't support:
    ///
    /// - sending more than one tokens at a time;
    /// - specifying a custom hook and hook metadata.
    ///
    /// These should be trivial to implement, but we just don't see a use for
    /// them for now.
    TransferRemote {
        destination_domain: Domain,
        // Note: This means the account the tokens are being sent to, NOT the
        // Hyperlane Warp contract, which is called "route" here and set by the
        // contract owner.
        recipient: Addr32,
        // Note: This is the metadata be to included in the [`TokenMessage`](crate::warp::TokenMessage),
        // NOT the metadata for the hooks.
        metadata: Option<HexBinary>,
    },
    /// Define the recipient contract and withdrawal fee rate for a token on a
    /// destination domain.
    SetRoute {
        denom: Denom,
        destination_domain: Domain,
        route: Route,
    },
    /// Set withdraw rate limits.
    SetRateLimits(BTreeMap<Denom, RateLimit>),
    /// Required Hyperlane recipient interface.
    Recipient(RecipientMsg),
}

#[grug::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    /// Query the address of the mailbox contract.
    #[returns(Addr)]
    Mailbox {},
    /// Query withdraw rate limits.
    #[returns(BTreeMap<Denom, RateLimit>)]
    RateLimits {},
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
    /// Query the remaining outbound quota for a denom.
    #[returns(Uint128)]
    OutboundQuota { denom: Denom },
    /// Enumerate all outbound quotas.
    #[returns(BTreeMap<Denom, Uint128>)]
    OutboundQuotas {
        start_after: Option<Denom>,
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
