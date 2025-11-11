use {
    super::{logarithm::ln_dec, *},
    grug::{Dec, Duration, Fraction},
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
    let value_of_inventory_in_base = Price::new(quote_inventory.into_inner())
        .checked_div(oracle_price)?
        .checked_add(Price::new(base_inventory.into_inner()))?;
    let base_inventory_target = base_inventory_target_percentage
        .into_inner()
        .checked_mul(value_of_inventory_in_base)?;

    Ok(oracle_price.checked_sub(
        base_inventory_target.checked_mul(
            Price::new(time_horizon.into_seconds())
                .checked_mul_dec(gamma)?
                .checked_mul_dec(sigma_squared)?,
        )?,
    )?)
}

pub fn half_spread(
    k: Price,
    gamma: Price,
    sigma_squared: Price,
    time_horizon: Duration,
) -> anyhow::Result<Price> {
    let one_over_k = k.checked_inv()?;
    let one_plus_gamma_over_k = Price::ONE.checked_add(gamma.checked_div(k)?)?;

    // Compute the natural logarithm of the one plus gamma over k.
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
