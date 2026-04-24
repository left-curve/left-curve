use {
    super::{Addr32, Origin, RateLimit, Remote},
    grug::{Addr, Denom, Duration, Op, Timestamp, Uint128},
    std::collections::{BTreeMap, BTreeSet},
};

#[grug::derive(Serde)]
pub struct WithdrawalFee {
    pub denom: Denom,
    pub remote: Remote,
    /// Use `Op::Insert` to add a new fee or change an existing fee; use
    /// `Op::Delete` to remove a fee.
    pub fee: Op<Uint128>,
}

/// Per-account allowance that is consumed before the global outbound quota
/// when the user sends a remote transfer. This is the stored / returned
/// form; `SetPersonalQuotaRequest` is the admin input.
#[grug::derive(Borsh, Serde)]
pub struct PersonalQuota {
    pub amount: Uint128,
    /// `None` means the quota never expires. `Some(t)` means the quota is
    /// ignored once the current block timestamp reaches `t`.
    pub expire_at: Option<Timestamp>,
    /// The admin account that created or last overwrote this entry.
    pub granted_by: Addr,
    /// The block timestamp of the grant or most recent overwrite.
    pub granted_at: Timestamp,
}

/// Admin input for `ExecuteMsg::SetPersonalQuota`. Carries the relative
/// lifetime `available_for`; the contract translates it into an absolute
/// `expiry` when saving the resulting `PersonalQuota`.
#[grug::derive(Serde)]
pub struct SetPersonalQuotaRequest {
    pub amount: Uint128,
    /// `None` means the quota never expires. `Some(d)` means the quota
    /// expires at `current_block_time + d`.
    pub available_for: Option<Duration>,
}

#[grug::derive(Serde)]
pub struct InstantiateMsg {
    pub routes: BTreeSet<(Origin, Addr, Remote)>,
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
    SetRoutes(BTreeSet<(Origin, Addr, Remote)>),

    /// Set rate limit for the routes.
    SetRateLimits(BTreeMap<Denom, RateLimit>),

    /// Set withdrawal fees for the denoms.
    SetWithdrawalFees(Vec<WithdrawalFee>),

    /// Grant or revoke a per-account, per-denom withdrawal allowance that is
    /// consumed before the global outbound quota.
    ///
    /// `Op::Insert(request)` overwrites any existing entry for the same
    /// `(user, denom)` with the fields in `request`. `Op::Delete` removes
    /// the entry entirely.
    ///
    /// Can only be called by the chain owner.
    SetPersonalQuota {
        user: Addr,
        denom: Denom,
        quota: Op<SetPersonalQuotaRequest>,
    },

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
    /// Enumerate all routes.
    #[returns(Vec<QueryRoutesResponseItem>)]
    Routes {
        start_after: Option<(Addr, Remote)>,
        limit: Option<u32>,
    },
    /// Query the withdraw rate limits.
    #[returns(BTreeMap<Denom, RateLimit>)]
    RateLimits {},
    /// Given a `(bridge, remote)` tuple, find the reserve amount.
    #[returns(Uint128)]
    Reserve { bridge: Addr, remote: Remote },
    /// Enumerate all reserves.
    #[returns(Vec<QueryReservesResponseItem>)]
    Reserves {
        start_after: Option<(Addr, Remote)>,
        limit: Option<u32>,
    },
    /// Given a `(denom, remote)` tuple, find the withdrawal fee.
    #[returns(Uint128)]
    WithdrawalFee { denom: Denom, remote: Remote },
    /// Enumerate all withdrawal fees.
    #[returns(Vec<QueryWithdrawalFeesResponseItem>)]
    WithdrawalFees {
        start_after: Option<(Denom, Remote)>,
        limit: Option<u32>,
    },
    /// Look up the personal quota an account has for a given denom.
    #[returns(Option<PersonalQuota>)]
    PersonalQuota { user: Addr, denom: Denom },
    /// Enumerate all personal quotas.
    #[returns(Vec<QueryPersonalQuotasResponseItem>)]
    PersonalQuotas {
        start_after: Option<(Addr, Denom)>,
        limit: Option<u32>,
    },
}

#[grug::derive(Serde)]
pub struct QueryRoutesResponseItem {
    pub bridge: Addr,
    pub remote: Remote,
    pub denom: Denom,
}

#[grug::derive(Serde)]
pub struct QueryReservesResponseItem {
    pub bridge: Addr,
    pub remote: Remote,
    pub reserve: Uint128,
}

#[grug::derive(Serde)]
pub struct QueryWithdrawalFeesResponseItem {
    pub denom: Denom,
    pub remote: Remote,
    pub fee: Uint128,
}

#[grug::derive(Serde)]
pub struct QueryPersonalQuotasResponseItem {
    pub user: Addr,
    pub denom: Denom,
    pub quota: PersonalQuota,
}
