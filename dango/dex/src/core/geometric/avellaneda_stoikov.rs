use {
    super::{math::ln_dec, *},
    grug::{Dec128_24, Dec256_24, Duration, Fraction, NextNumber, PrevNumber, Udec256_24}, std::str::FromStr,
};

/// Computes the reservation price for the Avellaneda-Stoikov model.
///
/// # Arguments
///
/// * `oracle_price` - The current price of the base asset in the quote asset.
/// * `base_inventory` - The current inventory of the base asset.
/// * `base_inventory_target_percentage` - The target inventory percentage of the base asset.
/// * `sigma_squared` - The squared volatility of the base asset.
/// * `gamma` - The gamma parameter of the Avellaneda-Stoikov model.
/// * `time_horizon` - The time horizon of the Avellaneda-Stoikov model.
///
/// # Returns
///
/// The reservation price for the Avellaneda-Stoikov model.
pub fn reservation_price(
    oracle_price: Price,
    base_inventory: Uint128,
    quote_inventory: Uint128,
    base_inventory_target_percentage: Bounded<Udec128, ZeroExclusiveOneExclusive>,
    sigma_squared: Price,
    gamma: Price,
    time_horizon: Duration,
) -> anyhow::Result<Price> {
    // Normalise the target inventory percentage to an amount of base asset.
    let value_of_inventory_in_base = quote_inventory.into_next()
        .checked_into_dec::<24>()?
        .checked_div(oracle_price.into_next())?
        .checked_add(Udec256_24::new(base_inventory.into_inner()))?;
    let base_inventory_target = base_inventory_target_percentage
        .into_inner()
        .into_next()
        .checked_mul(value_of_inventory_in_base)?;
    let base_inventory_diff_from_target = base_inventory
        .checked_into_dec::<24>()?
        .checked_into_signed()?
        .into_next()
        .checked_sub(
            base_inventory_target
                .checked_into_signed()?
        )?;

    let time_horizon_seconds = Dec128_24::new(time_horizon.into_seconds() as i128);

    let reservation_price = oracle_price.checked_into_signed()?.into_next().checked_sub(
        base_inventory_diff_from_target.checked_mul(
            time_horizon_seconds
                .checked_mul(gamma.checked_into_signed()?)?
                .checked_mul(sigma_squared.checked_into_signed()?)?.into_next(),
        )?,
    )?;

    let signed_oracle_price = oracle_price.checked_into_signed()?;
    let lower_bound = signed_oracle_price.checked_mul(Dec128_24::from_str("0.95").unwrap())?.into_next();
    let upper_bound = signed_oracle_price.checked_mul(Dec128_24::from_str("1.05").unwrap())?.into_next();
    let capped_reservation_price = reservation_price.max(lower_bound).min(upper_bound).checked_into_unsigned()?;

    Ok(capped_reservation_price.checked_into_prev()?)
}

pub fn half_spread(
    k: Price,
    gamma: Price,
    sigma_squared: Price,
    time_horizon: Duration,
) -> anyhow::Result<Price> {
    let one_over_k = k.checked_inv()?;
    let one_plus_gamma_over_k = Price::ONE.checked_add(gamma.checked_div(k)?)?;

    // Compute the natural logarithm of one plus gamma over k.
    let natural_log_of_one_plus_gamma_over_k =
        ln_dec(one_plus_gamma_over_k.checked_into_signed()?)?.checked_into_unsigned()?;

    let time_horizon_as_dec = Price::new(time_horizon.into_seconds());

    Ok(one_over_k
        .checked_mul(natural_log_of_one_plus_gamma_over_k)?
        .checked_add(
            gamma
                .checked_mul(sigma_squared)?
                .checked_mul(time_horizon_as_dec)?,
        )?)
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    use test_case::test_case;

    #[test_case(
        Price::ONE, 
        Price::ZERO,
        Price::ONE,
        Duration::from_seconds(300)
        => Price::ZERO
        ; "gamma = 0 means no spread"
    )]
    #[test_case(
        Price::ONE, 
        Price::ONE,
        Price::from_str("0.00000000076787914154").unwrap(),
        Duration::from_seconds(300)
        => Price::from_str("0.693377544302407308652726").unwrap()
    )]
    #[test_case(
        Price::ONE, 
        Price::ONE,
        Price::from_str("0.000000076787914154").unwrap(),
        Duration::from_seconds(300)
        => Price::from_str("0.716183554806145308652726").unwrap()
        ; "higher volatility means higher spread"
    )]
    fn test_half_spread(
        k: Price,
        gamma: Price,
        sigma_squared: Price,
        time_horizon: Duration,
    ) -> Price {
        println!("gamma: {:?}", gamma.to_string());
        half_spread(k, gamma, sigma_squared, time_horizon).unwrap()
    }

    #[test_case(
        Price::ONE, 
        Uint128::new(1_000_000_000u128),
        Uint128::new(1_000_000_000u128),
        Bounded::new(Udec128::from_str("0.5").unwrap()).unwrap(),
        Price::ONE,
        Price::ONE,
        Duration::from_seconds(120)
        => Price::ONE
    )]
    #[test_case(
        Price::from_str("1.1").unwrap(),
        Uint128::new(1_000_000_000u128),
        Uint128::new(1_000_000_000u128),
        Bounded::new(Udec128::from_str("0.5").unwrap()).unwrap(),
        Price::from_str("0.00000000076787914154").unwrap(),
        Price::from_str("0.01").unwrap(),
        Duration::from_seconds(120)
        => Price::from_str("1.058115683188727272727273").unwrap()
        ; "too high reserves moves reservation price below oracle price"
    )]
    #[test_case(
        Price::from_str("0.9").unwrap(), 
        Uint128::new(1_000_000_000u128),
        Uint128::new(1_000_000_000u128),
        Bounded::new(Udec128::from_str("0.5").unwrap()).unwrap(),
        Price::from_str("0.00000000076787914154").unwrap(),
        Price::from_str("0.01").unwrap(),
        Duration::from_seconds(120)
        => Price::from_str("0.951191942769333333333333").unwrap()
        ; "too low reserves moves reservation price above oracle price"
    )]
    fn test_reservation_price(
        oracle_price: Price,
        base_inventory: Uint128,
        quote_inventory: Uint128,
        base_inventory_target_percentage: Bounded<Udec128, ZeroExclusiveOneExclusive>,
        sigma_squared: Price,
        gamma: Price,
        time_horizon: Duration,
    ) -> Price {
        reservation_price(
            oracle_price,
            base_inventory,
            quote_inventory,
            base_inventory_target_percentage,
            sigma_squared,
            gamma,
            time_horizon,
        )
        .unwrap()
    }
}
