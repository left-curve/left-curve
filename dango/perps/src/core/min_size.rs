use {
    anyhow::ensure,
    dango_types::{Quantity, UsdPrice, perps::PairParam},
    grug::MathResult,
};

/// Ensure that the resulting position after applying `order_size` to
/// `current_position` meets the pair's minimum position size.
///
/// A resulting position of exactly zero (full close) is always allowed.
pub fn check_minimum_position_size(
    current_position: Quantity,
    order_size: Quantity,
    oracle_price: UsdPrice,
    pair_param: &PairParam,
) -> anyhow::Result<()> {
    let resulting_position = current_position.checked_add(order_size)?;
    let resulting_notional = resulting_position
        .checked_abs()?
        .checked_mul(oracle_price)?;

    // Full close (zero position) is always allowed.
    if resulting_notional.is_zero() {
        return Ok(());
    }

    ensure!(
        resulting_notional >= pair_param.min_position_size,
        "resulting position notional is below minimum: {} < {}",
        resulting_notional,
        pair_param.min_position_size
    );

    Ok(())
}

/// If closing `close_amount` of a position of absolute size `abs_size` would
/// leave a remainder whose notional is below `min_position_size`, snap to full
/// close.
///
/// Returns the (possibly increased) close amount.
pub fn snap_to_full_close(
    close_amount: Quantity,
    abs_size: Quantity,
    oracle_price: UsdPrice,
    pair_param: &PairParam,
) -> MathResult<Quantity> {
    if close_amount < abs_size {
        let remainder = abs_size.checked_sub(close_amount)?;
        let remainder_notional = remainder.checked_mul(oracle_price)?;
        if remainder_notional < pair_param.min_position_size {
            return Ok(abs_size);
        }
    }

    Ok(close_amount)
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {super::*, dango_types::UsdValue, test_case::test_case};

    // (current_pos, order_size, oracle_price, min_position_size, should_pass)
    #[test_case(  0,   1, 100, 100, true  ; "new long exactly at minimum")]
    #[test_case(  0,   2, 100, 100, true  ; "new long above minimum")]
    #[test_case(  0,   1, 100, 200, false ; "new long below minimum")]
    #[test_case(  0,  -1, 100, 100, true  ; "new short exactly at minimum")]
    #[test_case(  0,  -1, 100, 200, false ; "new short below minimum")]
    #[test_case( 10, -10, 100, 100, true  ; "full close long always allowed")]
    #[test_case(-10,  10, 100, 100, true  ; "full close short always allowed")]
    #[test_case( 10,  -9, 100, 200, false ; "partial close long leaves dust")]
    #[test_case( 10,  -9, 100,  50, true  ; "partial close long remainder above min")]
    #[test_case(-10,   9, 100, 200, false ; "partial close short leaves dust")]
    #[test_case( 10,   5, 100, 100, true  ; "increase long above min")]
    #[test_case(-10,  11, 100, 200, false ; "flip short to long dust")]
    #[test_case(-10,  12, 100, 100, true  ; "flip short to long above min")]
    fn check_minimum_position_size_works(
        current_pos: i128,
        order_size: i128,
        oracle_price: i128,
        min_position_size: i128,
        should_pass: bool,
    ) {
        assert_eq!(
            check_minimum_position_size(
                Quantity::new_int(current_pos),
                Quantity::new_int(order_size),
                UsdPrice::new_int(oracle_price),
                &PairParam {
                    min_position_size: UsdValue::new_int(min_position_size),
                    ..Default::default()
                }
            )
            .is_ok(),
            should_pass
        );
    }

    // (close_amount, abs_size, oracle_price, min_position_size, expected)
    #[test_case( 9, 10, 100, 200, 10 ; "remainder below min snaps to full close")]
    #[test_case( 5, 10, 100, 200,  5 ; "remainder above min no snap")]
    #[test_case(10, 10, 100, 200, 10 ; "already full close unchanged")]
    #[test_case( 0, 10, 100, 200,  0 ; "zero close no snap")]
    #[test_case( 9, 10, 100,   0,  9 ; "min zero disables snap")]
    fn snap_to_full_close_works(
        close_amount: i128,
        abs_size: i128,
        oracle_price: i128,
        min_position_size: i128,
        expected: i128,
    ) {
        let result = snap_to_full_close(
            Quantity::new_int(close_amount),
            Quantity::new_int(abs_size),
            UsdPrice::new_int(oracle_price),
            &PairParam {
                min_position_size: UsdValue::new_int(min_position_size),
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(result, Quantity::new_int(expected));
    }
}
