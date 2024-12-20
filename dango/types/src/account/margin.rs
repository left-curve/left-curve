use {
    crate::auth::Nonce,
    grug::{Bounded, Coins, Denom, Udec128, Uint128, ZeroExclusiveOneInclusive},
    std::collections::BTreeSet,
};

/// A decimal bounded by the bounds: 0 < CollateralPower <= 1.
pub type CollateralPower = Bounded<Udec128, ZeroExclusiveOneInclusive>;

/// The response type for a margin account's `Health` query.
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
    /// Queries the health of the margin account.
    #[returns(HealthResponse)]
    Health {},
}

#[grug::derive(Serde)]
pub struct LiquidationEvent {
    pub liquidation_denom: Denom,
    pub repay_coins: Coins,
    pub refunds: Coins,
    pub repaid_debt_value: Udec128,
    pub claimed_collateral: Uint128,
    pub liquidation_bonus: Udec128,
    pub target_health_factor: Udec128,
}
