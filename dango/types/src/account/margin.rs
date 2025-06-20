use {
    crate::{auth::Nonce, dex::OrdersByUserResponse},
    grug::{Bounded, Coins, Denom, Udec128, Udec256, Uint128, ZeroExclusiveOneInclusive},
    std::collections::{BTreeMap, BTreeSet},
};

/// A decimal bounded by the bounds: 0 < CollateralPower <= 1.
pub type CollateralPower = Bounded<Udec128, ZeroExclusiveOneInclusive>;

/// Necessary input data for computing a margin account's health.
#[grug::derive(Serde)]
pub struct HealthData {
    pub scaled_debts: BTreeMap<Denom, Udec256>,
    pub collateral_balances: BTreeMap<Denom, Uint128>,
    pub limit_orders: BTreeMap<u64, OrdersByUserResponse>,
}

/// Output for computing a margin account's health.
#[grug::derive(Serde)]
pub struct HealthResponse {
    /// The margin account's utilization rate.
    pub utilization_rate: Udec128,
    /// The total value of the margin account's debt.
    pub total_debt_value: Udec128,
    /// The total value of the margin account's collateral.
    pub total_collateral_value: Udec128,
    /// The total value of the margin account's collateral, adjusted for
    /// the collateral power of each denom.
    pub total_adjusted_collateral_value: Udec128,
    /// All of the accounts debts.
    pub debts: Coins,
    /// All of the account's collateral balances.
    pub collaterals: Coins,
    /// All of the account's collateral balances that are inside of limit orders.
    pub limit_order_collaterals: Coins,
    /// The coins that would be returned if the account's limit orders were to be filled.
    pub limit_order_outputs: Coins,
}

#[grug::derive(Serde)]
pub enum ExecuteMsg {
    /// Liquidate the margin account if it has become undercollateralized.
    Liquidate {
        /// The collateral denom to liquidate and be compensated with.
        collateral: Denom,
    },
}

/// Query messages for the margin account
#[grug::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    /// Query the most recent transaction nonces that have been recorded.
    #[returns(BTreeSet<Nonce>)]
    SeenNonces {},
    /// Query the data necessary for computing the account's health, but don't
    /// compute it yet.
    #[returns(HealthData)]
    HealthData {},
    /// Compute the health of the margin account.
    #[returns(Option<HealthResponse>)]
    Health {
        /// If the account has zero debt, then skip the rest of the computation
        /// involving collateral value and utilization rate, since the account
        /// is necessarily healthy if there is no debt.
        skip_if_no_debt: bool,
    },
}

#[grug::derive(Serde)]
#[grug::event("liquidate")]
pub struct Liquidate {
    pub collateral_denom: Denom,
    pub repay_coins: Coins,
    pub refunds: Coins,
    pub repaid_debt_value: Udec128,
    pub claimed_collateral_amount: Uint128,
    pub liquidation_bonus: Udec128,
    pub target_health_factor: Udec128,
}
