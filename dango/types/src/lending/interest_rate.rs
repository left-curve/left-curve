use grug::{
    Bounded, NumberConst, Udec128, ZeroExclusiveOneExclusive, ZeroInclusiveOneExclusive,
    ZeroInclusiveOneInclusive,
};

/// Dual slope intereate rate model, consisting of two linear functions.
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

/// Represents the calculated interest rates at a given utilization
#[derive(Debug)]
pub struct InterestRates {
    pub borrow_rate: Udec128,
    pub deposit_rate: Udec128,
}

impl InterestRateModel {
    /// Calculates interest rates for a given utilization rate
    pub fn calculate_rates(
        &self,
        utilization: Bounded<Udec128, ZeroInclusiveOneInclusive>,
    ) -> InterestRates {
        // Calculate borrow rate
        let borrow_rate = if *utilization <= *self.optimal_utilization {
            // Below optimal: linear increase
            *self.base_rate + (*utilization / *self.optimal_utilization) * *self.first_slope
        } else {
            // Above optimal: steeper increase
            let excess_utilization = (*utilization - *self.optimal_utilization)
                / (Udec128::ONE - *self.optimal_utilization);
            *self.base_rate + *self.first_slope + (excess_utilization * *self.second_slope)
        };

        // Calculate deposit rate
        let deposit_rate = *utilization * borrow_rate * (Udec128::ONE - *self.reserve_factor);

        InterestRates {
            borrow_rate,
            deposit_rate,
        }
    }
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

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_params() {
        let model = InterestRateModel::default();
        assert_eq!(*model.optimal_utilization, Udec128::new_percent(80));
        assert_eq!(*model.first_slope, Udec128::new_percent(4));
        assert_eq!(*model.second_slope, Udec128::new_percent(75));
        assert_eq!(*model.reserve_factor, Udec128::new_percent(2));
    }

    #[test]
    fn test_zero_utilization() {
        let model = InterestRateModel::default();
        let rates = model.calculate_rates(Bounded::new_unchecked(Udec128::ZERO));
        assert_eq!(rates.borrow_rate, *model.base_rate);
        assert_eq!(rates.deposit_rate, Udec128::ZERO);
    }

    #[test]
    fn test_max_utilization() {
        let model = InterestRateModel::default();
        let rates = model.calculate_rates(Bounded::new_unchecked(Udec128::ONE));
        assert_eq!(
            rates.borrow_rate,
            *model.base_rate + *model.first_slope + *model.second_slope
        );
    }
}
