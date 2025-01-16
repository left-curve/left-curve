use {
    crate::auth::Nonce,
    grug::{Bound, Bounded, Bounds, NumberConst, Udec128},
    std::collections::BTreeSet,
};

/// Defines the bounds for a collateral power: 0 < CollateralPower <= 1.
#[grug::derive(Serde)]
pub struct CollateralPowerBounds;

impl Bounds<Udec128> for CollateralPowerBounds {
    const MAX: Option<Bound<Udec128>> = Some(Bound::Inclusive(Udec128::ONE));
    const MIN: Option<Bound<Udec128>> = Some(Bound::Exclusive(Udec128::ZERO));
}

/// A decimal bounded by the collateral power bounds.
pub type CollateralPower = Bounded<Udec128, CollateralPowerBounds>;

/// The response type for a margin account's `Health` query.
#[grug::derive(Serde)]
pub struct HealthResponse {
    /// The margin account's utilization rate.
    pub utilization_rate: Udec128,
    /// The total value of the margin account's debt.
    pub total_debt_value: Udec128,
    /// The total value of the margin account's collateral, adjusted for
    /// the collateral power of each denom.
    pub total_adjusted_collateral_value: Udec128,
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
