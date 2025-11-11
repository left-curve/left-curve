use {
    dango_types::dex::Price,
    grug::{
        Denom, Exponentiate, Map, Number, NumberConst, Signed, Storage, Timestamp, Udec128,
        Unsigned,
    },
};

use crate::core::geometric::logarithm::ln_interpolated_one_to_two;
const LAST_PRICE: Map<(&Denom, &Denom), (Timestamp, Price)> = Map::new("last_price");
pub const LAST_VOLATILITY_ESTIMATE: Map<(&Denom, &Denom), Price> =
    Map::new("last_volatility_estimate");

/// Updates the volatility estimate using an exponential moving average:
///
/// vol_estimate_t = lambda * vol_estimate_{t-1}^2 + (1 - lambda) * r_t^2
///
/// where vol_estimate_t is the volatility estimate at time t, and vol_estimate_{t-1} is the
/// volatility estimate at time t-1 and r_t is the log return at time t.
///
/// This function also saves the last price and squared volatility estimate to storage.
///
/// # Arguments
///
/// * `storage` - The storage to save the last price and volatility estimate.
/// * `pair_id` - The pair id for which to update the volatility estimate.
/// * `price` - The latest price for the pair.
/// * `lambda` - The decay rate used for the smoothing of the volatility estimate.
///
/// # Returns
///
/// The updated volatility estimate.
pub fn update_volatility_estimate(
    storage: &mut dyn Storage,
    block_time: Timestamp,
    base_denom: &Denom,
    quote_denom: &Denom,
    price: Price,
    lambda: Price,
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
    let r_t_squared =
        ln_interpolated_one_to_two(price.checked_div(last_price)?.checked_into_signed()?)?
            .checked_pow(2)?
            .checked_into_unsigned()?;

    // Normalise the log return to the time interval
    let r_t_squared_norm = r_t_squared.checked_div(Udec128::new(
        block_time.checked_sub(last_timestamp)?.into_seconds(),
    ))?;

    let vol_estimate = lambda.checked_mul(prev_squared_vol)?.checked_add(
        Price::ONE
            .checked_sub(lambda)?
            .checked_mul(r_t_squared_norm)?,
    )?;

    // Save the last price and squared volatility estimate to storage
    LAST_PRICE.save(storage, (base_denom, quote_denom), &(block_time, price))?;
    LAST_VOLATILITY_ESTIMATE.save(storage, (base_denom, quote_denom), &vol_estimate)?;

    Ok(vol_estimate)
}
