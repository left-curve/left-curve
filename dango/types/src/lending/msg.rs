use {
    crate::lending::{InterestRateModel, Market},
    grug::{Addr, Coins, Denom, NonEmpty, Part},
    std::{collections::BTreeMap, sync::LazyLock},
};

/// The namespace that tokens associated with lending will be minted under.
/// The lending contract must be granted admin power over this namespace.
pub static NAMESPACE: LazyLock<Part> = LazyLock::new(|| Part::new_unchecked("lending"));

/// Sub-namespace that liquidity share tokens will be minted under.
pub static SUBNAMESPACE: LazyLock<Part> = LazyLock::new(|| Part::new_unchecked("pool"));

#[grug::derive(Serde)]
pub struct InstantiateMsg {
    pub markets: BTreeMap<Denom, InterestRateModel>,
}

#[grug::derive(Serde)]
pub enum ExecuteMsg {
    /// Apply updates to markets.
    UpdateMarkets(BTreeMap<Denom, InterestRateModel>),
    /// Deposit tokens into the lending pool.
    /// Sender must attach one or more supported tokens and nothing else.
    Deposit {},
    /// Withdraw tokens from the lending pool by redeeming LP tokens.
    /// Sender must attach one or more LP tokens and nothing else.
    Withdraw {},
    /// Borrow coins from the lending pool.
    /// Sender must be a margin account.
    Borrow(NonEmpty<Coins>),
    /// Repay debt.
    /// Sender must be a margin account.
    Repay {},
    /// Claim pending protocol fees for a range of denoms.
    ClaimPendingProtocolFees {},
}

#[grug::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    /// Query the lending market of a single token.
    #[returns(Market)]
    Market { denom: Denom },
    /// Enumerate all lending markets.
    #[returns(BTreeMap<Denom, Market>)]
    Markets {
        start_after: Option<Denom>,
        limit: Option<u32>,
    },
    /// Query the debt of a single margin account.
    #[returns(Coins)]
    Debt { account: Addr },
    /// Enumerate debts of all margin accounts.
    #[returns(BTreeMap<Addr, Coins>)]
    Debts {
        start_after: Option<Addr>,
        limit: Option<u32>,
    },
    /// Converts the supplied amount of underlying tokens to LP tokens.
    #[returns(Coins)]
    PreviewDeposit { underlying: Coins },
    /// Converts the supplied amount of LP tokens to the underlying tokens.
    #[returns(Coins)]
    PreviewWithdraw { lp_tokens: Coins },
}
