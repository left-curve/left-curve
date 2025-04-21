use {
    dango_types::lending::InterestRateModel,
    grug::{Bounded, NumberConst, Udec128, ZeroInclusiveOneInclusive},
};

/// Calculates borrow and supply interest rates based on a given `Market`'s
/// utilization rate.
///
/// ## Inputs
///
/// - `utilization`: The current market utilization rate. Must be within the
///   range [0, 1].
///
/// ## Outputs
///
/// - The borrow interest rate.
/// - The supply interest rate.
pub fn calculate_rates(
    model: &InterestRateModel,
    utilization: Bounded<Udec128, ZeroInclusiveOneInclusive>,
) -> (Udec128, Udec128) {
    // Calculate borrow rate
    let borrow_rate = if *utilization <= *model.optimal_utilization {
        // Below optimal: linear increase
        *model.base_rate + (*utilization / *model.optimal_utilization) * *model.first_slope
    } else {
        // Above optimal: steeper increase
        let excess_utilization = (*utilization - *model.optimal_utilization)
            / (Udec128::ONE - *model.optimal_utilization);
        *model.base_rate + *model.first_slope + (excess_utilization * *model.second_slope)
    };

    // Calculate deposit rate
    let supply_rate = *utilization * borrow_rate * (Udec128::ONE - *model.reserve_factor);

    (borrow_rate, supply_rate)
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zero_utilization() {
        let model = InterestRateModel::default();
        let utilization = Bounded::new_unchecked(Udec128::ZERO);
        let (borrow_rate, supply_rate) = calculate_rates(&model, utilization);
        assert_eq!(borrow_rate, *model.base_rate);
        assert_eq!(supply_rate, Udec128::ZERO);
    }

    #[test]
    fn test_max_utilization() {
        let model = InterestRateModel::default();
        let utilization = Bounded::new_unchecked(Udec128::ONE);
        let (borrow_rate, _) = calculate_rates(&model, utilization);
        assert_eq!(
            borrow_rate,
            *model.base_rate + *model.first_slope + *model.second_slope
        );
        // TODO: also check supply rate
    }
}
