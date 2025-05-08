use {
    super::{Addr32, RateLimit, Remote},
    grug::{Addr, Denom, Part, Uint128},
    std::collections::{BTreeMap, BTreeSet},
};

#[grug::derive(Serde)]
pub struct WithdrawalFee {
    pub denom: Denom,
    pub remote: Remote,
    pub fee: Uint128,
}

#[grug::derive(Serde)]
pub struct InstantiateMsg {
    pub routes: BTreeSet<(Part, Addr, Remote)>,
    pub rate_limits: BTreeMap<Denom, RateLimit>,
    pub withdrawal_fees: Vec<WithdrawalFee>,
}

#[grug::derive(Serde)]
pub enum ExecuteMsg {
    /// Create new routes.
    ///
    /// Can only be called by the chain owner.
    ///
    /// Not that this is append-only, meaning you can't change or remove an
    /// existing route.
    SetRoutes(BTreeSet<(Part, Addr, Remote)>),
    /// Set rate limit for the routes.
    SetRateLimits(BTreeMap<Denom, RateLimit>),
    /// Set withdrawal fees for the denoms.
    SetWithdrawalFees(Vec<WithdrawalFee>),
    /// Receive a token transfer from a remote chain.
    ///
    /// Can only be called by contracts for which has been assigned a
    ReceiveRemote {
        remote: Remote,
        amount: Uint128,
        recipient: Addr,
    },
    /// Send a token transfer to a remote chain.
    ///
    /// Can be called by anyone.
    TransferRemote { remote: Remote, recipient: Addr32 },
}

#[grug::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    /// Given a `(bridge, remote)` tuple, find the alloyed denom it belongs to.
    #[returns(Option<Denom>)]
    Route { bridge: Addr, remote: Remote },
    /// Given an alloyed denom and the remote, find the bridge contract that handles it.
    #[returns(Option<Addr>)]
    ReverseRoute { denom: Denom, remote: Remote },
    /// Query the withdraw rate limits.
    #[returns(BTreeMap<Denom, RateLimit>)]
    RateLimits {},
}
