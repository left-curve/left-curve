use std::fmt::Display;

use {
    anyhow::anyhow,
    grug::{
        Bounded, NumberConst, Udec128, ZeroExclusiveOneExclusive, ZeroInclusiveOneExclusive,
        ZeroInclusiveOneInclusive,
    },
};

/// Defines different interest rate models (calculates how much interest is paid
/// by borrowers depending on current market utilization).
#[grug::derive(Serde, Borsh)]
#[non_exhaustive]
pub enum InterestRateModel {
    /// An interest rate model consisting of two linear functions. This is based
    /// on Aave's interest rate model. The first slope is applied when the
    /// utilization is below the optimal utilization rate, and the second slope
    /// is applied when the utilization is above the optimal utilization rate.
    DualSlope {
        /// The base interest rate. This is the interest rate that is applied
        /// when the utilization is 0%.
        base_rate: Bounded<Udec128, ZeroInclusiveOneExclusive>,
        /// The optimal utilization rate. This is the utilization rate after
        /// which the second slope is applied.
        optimal_utilization: Bounded<Udec128, ZeroExclusiveOneExclusive>,
        /// The slope of the first linear function. This is the slope that is
        /// applied when the utilization is below the optimal utilization rate.
        first_slope: Bounded<Udec128, ZeroExclusiveOneExclusive>,
        /// The slope of the second linear function. This is the slope that is
        /// applied when the utilization is above the optimal utilization rate.
        second_slope: Bounded<Udec128, ZeroExclusiveOneExclusive>,
        /// The portion of interest retained as protocol reserves.
        reserve_factor: Bounded<Udec128, ZeroInclusiveOneInclusive>,
    },
}

/// Represents the calculated interest rates at a given utilization
#[derive(Debug)]
pub struct InterestRates {
    pub borrow_rate: Udec128,
    pub deposit_rate: Udec128,
    pub spread: Udec128,
}

impl Display for InterestRates {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "borrow_rate: {}, deposit_rate: {}, spread: {}",
            self.borrow_rate, self.deposit_rate, self.spread
        )
    }
}

impl InterestRateModel {
    /// Calculates interest rates for a given utilization rate
    pub fn calculate_rates(&self, utilization: Udec128) -> anyhow::Result<InterestRates> {
        match self {
            Self::DualSlope {
                base_rate,
                optimal_utilization,
                first_slope,
                second_slope,
                reserve_factor,
            } => {
                if utilization > Udec128::new_percent(100) {
                    return Err(anyhow!("invalid utilization rate"));
                }

                // Calculate borrow rate
                let borrow_rate = if utilization <= **optimal_utilization {
                    // Below optimal: linear increase
                    **base_rate + (utilization / **optimal_utilization) * **first_slope
                } else {
                    // Above optimal: steeper increase
                    let excess_utilization = (utilization - **optimal_utilization)
                        / (Udec128::ONE - **optimal_utilization);
                    **base_rate + **first_slope + (excess_utilization * **second_slope)
                };

                // Calculate deposit rate
                let deposit_rate = utilization * borrow_rate * (Udec128::ONE - **reserve_factor);

                // Calculate spread
                let spread = borrow_rate - deposit_rate;

                Ok(InterestRates {
                    borrow_rate,
                    deposit_rate,
                    spread,
                })
            },
        }
    }

    /// Returns the reserve factor for the interest rate model.
    pub fn reserve_factor(&self) -> Udec128 {
        match self {
            Self::DualSlope { reserve_factor, .. } => **reserve_factor,
        }
    }
}

impl Default for InterestRateModel {
    /// Default interest rate model used for testing.
    fn default() -> Self {
        Self::DualSlope {
            base_rate: Bounded::new(Udec128::new_percent(1)).unwrap(),
            optimal_utilization: Bounded::new(Udec128::new_percent(80)).unwrap(),
            first_slope: Bounded::new(Udec128::new_percent(4)).unwrap(),
            second_slope: Bounded::new(Udec128::new_percent(75)).unwrap(),
            reserve_factor: Bounded::new(Udec128::new_percent(2)).unwrap(),
        }
    }
}

#[cfg(test)]
mod tests {
    use {super::*, grug::ResultExt, plotters::prelude::*};

    #[test]
    fn test_default_params() {
        let model = InterestRateModel::default();
        match model {
            InterestRateModel::DualSlope {
                optimal_utilization,
                first_slope,
                second_slope,
                reserve_factor,
                ..
            } => {
                assert_eq!(*optimal_utilization, Udec128::new_percent(80));
                assert_eq!(*first_slope, Udec128::new_percent(4));
                assert_eq!(*second_slope, Udec128::new_percent(75));
                assert_eq!(*reserve_factor, Udec128::new_percent(2));
            },
        }
    }

    #[test]
    fn test_zero_utilization() {
        let model = InterestRateModel::default();
        match &model {
            InterestRateModel::DualSlope { base_rate, .. } => {
                let rates = model.calculate_rates(Udec128::ZERO).unwrap();
                assert_eq!(rates.borrow_rate, **base_rate);
                assert_eq!(rates.deposit_rate, Udec128::ZERO);
                assert_eq!(rates.spread, **base_rate);
            },
        }
    }

    #[test]
    fn test_max_utilization() {
        let model = InterestRateModel::default();
        match &model {
            InterestRateModel::DualSlope {
                base_rate,
                first_slope,
                second_slope,
                ..
            } => {
                let rates = model.calculate_rates(Udec128::ONE).unwrap();
                assert_eq!(
                    rates.borrow_rate,
                    **base_rate + **first_slope + **second_slope
                );
                assert!(rates.spread > Udec128::ZERO);
            },
        }
    }

    #[test]
    fn test_invalid_utilization() {
        InterestRateModel::default()
            .calculate_rates(Udec128::new_percent(110))
            .should_fail_with_error("invalid utilization rate");
    }

    #[test]
    fn plot_rates() {
        let model = InterestRateModel::default();

        match model {
            InterestRateModel::DualSlope { .. } => {
                let root = BitMapBackend::new("rates.png", (1024, 768)).into_drawing_area();
                root.fill(&WHITE).unwrap();

                let mut chart = ChartBuilder::on(&root)
                    .caption("Interest Rates", ("sans-serif", 25))
                    .x_label_area_size(50)
                    .y_label_area_size(50)
                    .build_cartesian_2d(0f32..100f32, 0f32..100f32)
                    .unwrap();

                chart
                    .configure_mesh()
                    .x_desc("Utilization (%)")
                    .y_desc("Interest Rate (%)")
                    .draw()
                    .unwrap();

                // Borrow rate line
                chart
                    .draw_series(LineSeries::new(
                        (0..100).map(|x| {
                            (
                                x as f32,
                                model
                                    .calculate_rates(Udec128::new_percent(x as u128))
                                    .unwrap()
                                    .borrow_rate
                                    .to_string()
                                    .parse::<f32>()
                                    .unwrap()
                                    * 100.0,
                            )
                        }),
                        &RED,
                    ))
                    .unwrap()
                    .label("Borrow Rate")
                    .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], RED));

                // Deposit rate line
                chart
                    .draw_series(LineSeries::new(
                        (0..100).map(|x| {
                            (
                                x as f32,
                                model
                                    .calculate_rates(Udec128::new_percent(x as u128))
                                    .unwrap()
                                    .deposit_rate
                                    .to_string()
                                    .parse::<f32>()
                                    .unwrap()
                                    * 100.0,
                            )
                        }),
                        &BLUE,
                    ))
                    .unwrap()
                    .label("Deposit Rate")
                    .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], BLUE));

                chart
                    .configure_series_labels()
                    .background_style(WHITE.mix(0.8))
                    .border_style(BLACK)
                    .draw()
                    .unwrap();

                root.present().unwrap();
            },
        }
    }
}
