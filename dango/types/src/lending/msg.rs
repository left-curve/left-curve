use {
    crate::lending::{InterestRateModel, Market},
    grug::{Addr, Coins, Denom, NonEmpty},
    std::collections::BTreeMap,
};

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
    /// Withdraw tokens from the lending pool.
    Withdraw(NonEmpty<Coins>),
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
    /// Query the deposit of a single lender.
    #[returns(Coins)]
    Asset { account: Addr },
    /// Enumerate deposits of all lenders.
    #[returns(BTreeMap<Addr, Coins>)]
    Assets {
        start_after: Option<Addr>,
        limit: Option<u32>,
    },
    /// Query the debt of a single borrower.
    #[returns(Coins)]
    Debt { account: Addr },
    /// Enumerate debts of all borrowers.
    #[returns(BTreeMap<Addr, Coins>)]
    Debts {
        start_after: Option<Addr>,
        limit: Option<u32>,
    },
}
