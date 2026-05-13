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

    /// Overwrite the global rate-limit configuration.
    ///
    /// The map is the new complete set of rate-limited denoms:
    ///
    /// - A denom absent from the map is not rate-limited — outbound
    ///   transfers of it are unrestricted (only reserves and withdrawal
    ///   fees still apply). Any existing supply snapshot and trailing-
    ///   window history for a dropped denom are cleared in the same block.
    /// - A denom mapped to `0` is fully locked — the cap is set to zero,
    ///   so no outbound transfer passes until the admin raises the limit
    ///   or removes the denom.
    /// - A denom mapped to a positive fraction less than `1` is enforced
    ///   as a trailing-24h cap: a withdraw is rejected when the sum of
    ///   withdraws over the trailing 24 hours plus the new request would
    ///   exceed `supply_snapshot × limit`. The supply snapshot is taken
    ///   by the cron handler once per refresh period and is also seeded
    ///   on the first `SetRateLimits` that adds the denom.
    ///
    /// A configured-limit change takes effect immediately on the next
    /// withdraw (`cap` rises or falls with `limit`), but the supply
    /// snapshot is not refreshed by this call — it only moves at cron
    /// ticks, so deposits between cron ticks cannot enlarge the cap.
    ///
    /// Can only be called by the chain owner.
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

    /// Look up the rate-limit status for a denom: the supply snapshot, the
    /// derived cap, and the trailing-24h withdraw volume. Returns `None` if
    /// the denom is not rate-limited.
    #[returns(Option<RateLimitStatus>)]
    RateLimitStatus { denom: Denom },

    /// Enumerate the rate-limit status for every rate-limited denom.
    #[returns(Vec<RateLimitStatusItem>)]
    RateLimitStatuses {
        start_after: Option<Denom>,
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

/// Rate-limit status for a single rate-limited denom: the supply snapshot
/// taken at the last cron tick (or at the denom's first registration), the
/// derived cap `supply_snapshot × limit`, and the rolling sum of withdraws
/// over the trailing 24 hours. Available headroom is `cap − used_in_last_24h`.
#[grug::derive(Serde)]
pub struct RateLimitStatus {
    pub supply_snapshot: Uint128,
    pub cap: Uint128,
    pub used_in_last_24h: Uint128,
}

#[grug::derive(Serde)]
pub struct RateLimitStatusItem {
    pub denom: Denom,
    pub status: RateLimitStatus,
}
