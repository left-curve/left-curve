use {
    super::{DestinationAddr, DestinationChain, RateLimit},
    grug::{Addr, Coin, Denom, Uint128},
    std::collections::BTreeMap,
};

#[grug::derive(Serde)]
pub struct InstantiateMsg {}

#[grug::derive(Serde)]
pub enum ExecuteMsg {
    RegisterDenom {
        denom: Denom,
        bridge_addr: Addr,
    },
    /// Register an alloyed token.
    SetAlloy {
        underlying_denom: Denom,
        destination_chain: DestinationChain,
        alloyed_denom: Denom,
    },
    /// Set withdraw rate limits.
    SetRateLimits(BTreeMap<Denom, RateLimit>),
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
    ///
    /// These should be trivial to implement, but we just don't see a use for
    /// them for now.
    TransferRemote {
        destination_chain: DestinationChain,
        recipient: DestinationAddr,
    },
    ReceiveRemote {
        token: Coin,
        recipient: Addr,
    },
}

#[grug::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    /// Query withdraw rate limits.
    #[returns(BTreeMap<Denom, RateLimit>)]
    RateLimits {},
    /// Query the alloyed denom corresponding to an underlying denom.
    #[returns(Denom)]
    Alloy { underlying_denom: Denom },
    /// Enumerate all alloyed denoms.
    #[returns(BTreeMap<Denom, Denom>)]
    Alloys {
        start_after: Option<Denom>,
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
}

#[grug::derive(Serde)]
pub enum BridgeHookMsg {
    HookTransferRemote(HookTransferRemote),
}

#[grug::derive(Serde)]
pub struct HookTransferRemote {
    pub token: Coin,
    pub destination_chain: DestinationChain,
    pub recipient: DestinationAddr,
}
