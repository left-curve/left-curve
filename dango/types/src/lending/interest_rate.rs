use grug::{
    Bounded, Udec128, ZeroExclusiveOneExclusive, ZeroInclusiveOneExclusive,
    ZeroInclusiveOneInclusive,
};

/// Dual slope interest rate model, consisting of two linear functions.
///
/// This is based on Aave's interest rate model. The first slope is applied when
/// the utilization is below the optimal utilization rate, and the second slope
/// is applied when the utilization is above the optimal utilization rate.
#[grug::derive(Serde, Borsh)]
pub struct InterestRateModel {
    /// The base interest rate. This is the interest rate that is applied
    /// when the utilization is 0%.
    pub base_rate: Bounded<Udec128, ZeroInclusiveOneExclusive>,
    /// The optimal utilization rate. This is the utilization rate after
    /// which the second slope is applied.
    pub optimal_utilization: Bounded<Udec128, ZeroExclusiveOneExclusive>,
    /// The slope of the first linear function. This is the slope that is
    /// applied when the utilization is below the optimal utilization rate.
    pub first_slope: Bounded<Udec128, ZeroExclusiveOneExclusive>,
    /// The slope of the second linear function. This is the slope that is
    /// applied when the utilization is above the optimal utilization rate.
    pub second_slope: Bounded<Udec128, ZeroExclusiveOneExclusive>,
    /// The portion of interest retained as protocol reserves.
    pub reserve_factor: Bounded<Udec128, ZeroInclusiveOneInclusive>,
}

impl Default for InterestRateModel {
    /// Default interest rate model used for testing.
    fn default() -> Self {
        Self {
            base_rate: Bounded::new(Udec128::new_percent(1)).unwrap(),
            optimal_utilization: Bounded::new(Udec128::new_percent(80)).unwrap(),
            first_slope: Bounded::new(Udec128::new_percent(4)).unwrap(),
            second_slope: Bounded::new(Udec128::new_percent(75)).unwrap(),
            reserve_factor: Bounded::new(Udec128::new_percent(2)).unwrap(),
        }
    }
}
