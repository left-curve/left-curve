use {
    crate::core::geometric::math::{NATURAL_LOG_OF_TWO, UnsignedDecimalConstant, e_pow},
    dango_types::dex::Price,
    grug::{
        Dec128, Denom, Duration, Exponentiate, Map, Number, NumberConst, Signed, Storage,
        Timestamp, Udec128, Unsigned,
    },
};

use crate::core::geometric::math::ln_dec;
const LAST_PRICE: Map<(&Denom, &Denom), (Timestamp, Price)> = Map::new("last_price");
pub const LAST_VOLATILITY_ESTIMATE: Map<(&Denom, &Denom), Price> =
    Map::new("last_volatility_estimate");

/// Updates the volatility estimate using an exponential moving average:
///
/// vol_estimate_t = (1 - alpha) * vol_estimate_{t-1}^2 + alpha * r_t^2
///
/// where vol_estimate_t is the volatility estimate at time t, vol_estimate_{t-1} is the
/// volatility estimate at time t-1, r_t is the log return at time t, and alpha is the
/// time-adaptive decay factor that increases with dt/half_life.
///
/// This function also saves the last price and squared volatility estimate to storage.
///
/// # Arguments
///
/// * `storage` - The storage to save the last price and volatility estimate.
/// * `pair_id` - The pair id for which to update the volatility estimate.
/// * `price` - The latest price for the pair.
/// * `half_life` - The half life of the weight of each sample in the volatility estimate.
///
/// # Returns
///
/// The updated volatility estimate in units of per millisecond squared.
pub fn update_volatility_estimate(
    storage: &mut dyn Storage,
    block_time: Timestamp,
    base_denom: &Denom,
    quote_denom: &Denom,
    price: Price,
    half_life: Duration,
) -> anyhow::Result<Price> {
    let (last_timestamp, last_price) = match LAST_PRICE
        .may_load(storage, (base_denom, quote_denom))?
    {
        Some(price) => price,
        None => {
            LAST_PRICE.save(storage, (base_denom, quote_denom), &(block_time, price))?;
            LAST_VOLATILITY_ESTIMATE.save(storage, (&base_denom, &quote_denom), &Price::ZERO)?;
            return Ok(Price::ZERO);
        },
    };

    let prev_squared_vol =
        match LAST_VOLATILITY_ESTIMATE.may_load(storage, (base_denom, quote_denom))? {
            Some(squared_vol) => squared_vol,
            None => {
                LAST_VOLATILITY_ESTIMATE.save(storage, (base_denom, quote_denom), &Price::ZERO)?;
                return Ok(Price::ZERO);
            },
        };

    // Compute the log return squared
    let r_t_squared = ln_dec(price.checked_div(last_price)?.checked_into_signed()?)?
        .checked_pow(2)?
        .checked_into_unsigned()?;

    // Compute the time diff since the last update in milliseconds
    let time_diff_ms = block_time.checked_sub(last_timestamp)?.into_millis();

    // Normalise the log return to one second time interval
    let r_t_squared_norm =
        r_t_squared.checked_div(Udec128::checked_from_ratio(time_diff_ms, 1000)?)?;

    // Calculate the decay rate for the sample based on the time diff
    // alpha = 1 - exp(-ln(2) * dt / half_life) = 1 - 1/exp(ln(2) * dt / half_life)
    let ln_of_two = NATURAL_LOG_OF_TWO::to_decimal_value::<24>()?;
    let alpha = Price::ONE.checked_sub(Price::ONE.checked_div(e_pow(
        Price::checked_from_ratio(time_diff_ms, half_life.into_millis())?.checked_mul(ln_of_two)?,
    )?)?)?;

    // Calculate the volatility estimate as
    // vol_estimate_t = (1 - alpha) * vol_estimate_{t-1}^2 + alpha * r_t^2
    let one_minus_alpha = Price::ONE.checked_sub(alpha)?;
    let term1 = one_minus_alpha.checked_mul(prev_squared_vol)?;
    let term2 = alpha.checked_mul(r_t_squared_norm)?;
    let vol_estimate = term1.checked_add(term2)?;

    // Save the last price and squared volatility estimate to storage
    LAST_PRICE.save(storage, (base_denom, quote_denom), &(block_time, price))?;
    LAST_VOLATILITY_ESTIMATE.save(storage, (base_denom, quote_denom), &vol_estimate)?;

    Ok(vol_estimate)
}

#[cfg(test)]
mod tests {
    use {super::*, grug::MockStorage};

    #[test]
    fn test_volatility_estimator_initial_conditions() {
        let mut storage = MockStorage::new();
        let base_denom = Denom::new_unchecked(vec!["base".to_string()]);
        let quote_denom = Denom::new_unchecked(vec!["quote".to_string()]);
        let price = Price::checked_from_atomics(100u128, 0).unwrap();

        // First call should return zero and initialize storage
        let estimate = update_volatility_estimate(
            &mut storage,
            Timestamp::from_seconds(0),
            &base_denom,
            &quote_denom,
            price,
            Duration::from_seconds(15),
        )
        .unwrap();

        assert_eq!(estimate, Price::ZERO);

        // Verify storage was initialized
        let (saved_time, saved_price) = LAST_PRICE
            .load(&storage, (&base_denom, &quote_denom))
            .unwrap();
        assert_eq!(saved_time, Timestamp::from_seconds(0));
        assert_eq!(saved_price, price);

        let saved_vol = LAST_VOLATILITY_ESTIMATE
            .load(&storage, (&base_denom, &quote_denom))
            .unwrap();
        assert_eq!(saved_vol, Price::ZERO);
    }
}
