use {
    super::{logarithm::ln_dec, *},
    grug::{Dec, Duration, Fraction, Unsigned},
};

pub fn reservation_price(
    oracle_price: Dec<i128, 24>,
    base_inventory: Uint128,
    sigma_squared: Dec<i128, 24>,
    gamma: Dec<i128, 24>,
    time_horizon: Duration,
) -> anyhow::Result<Dec<i128, 24>> {
    Ok(oracle_price.checked_sub(
        base_inventory
            .checked_mul(time_horizon.into_seconds().into())?
            .checked_into_dec::<24>()?
            .checked_into_signed()?
            .checked_mul_dec(gamma)?
            .checked_mul_dec(sigma_squared)?,
    )?)
}

pub fn half_spread(
    k: Dec<i128, 24>,
    gamma: Dec<i128, 24>,
    sigma_squared: Dec<i128, 24>,
    time_horizon: Duration,
) -> anyhow::Result<Dec<i128, 24>> {
    let one_over_k = k.checked_inv()?;
    let one_plus_gamma_over_k = Dec::<i128, 24>::ONE.checked_add(gamma.checked_div(k)?)?;

    // Compute the natural logarithm of the one plus gamma over k.
    let natural_log_of_one_plus_gamma_over_k = ln_dec(one_plus_gamma_over_k)?;

    let time_horizon_as_dec = Dec::<i128, 24>::new(time_horizon.into_seconds() as i128);

    Ok(one_over_k
        .checked_mul(natural_log_of_one_plus_gamma_over_k)?
        .checked_add(
            gamma
                .checked_mul(sigma_squared)?
                .checked_mul(time_horizon_as_dec)?,
        )?)
}
