use {
    crate::{Quantity, UsdPrice, UsdValue},
    dango_primitives::StdResult,
};

/// Walk an ordered sequence of `(limit_price, size)` pairs and compute the
/// volume-weighted average execution price for filling `impact_size` worth
/// of notional value.
///
/// Each item is `(limit_price, absolute_order_size)`. The caller is responsible
/// for iterating the correct side of the book (bids or asks) in price-priority
/// order and mapping storage entries to `(real_price, absolute_size)`.
///
/// Returns: `Some(vwap)` if enough depth exists, `None` otherwise.
pub fn compute_impact_price<I>(orders: I, impact_size: UsdValue) -> StdResult<Option<UsdPrice>>
where
    I: Iterator<Item = StdResult<(UsdPrice, Quantity)>>,
{
    let mut total_size = Quantity::ZERO;
    let mut total_notional = UsdValue::ZERO;

    for result in orders {
        let (price, size) = result?;
        let order_notional = size.checked_mul(price)?;
        let remaining = impact_size.checked_sub(total_notional)?;

        if order_notional >= remaining {
            let partial_size = remaining.checked_div(price)?;

            total_size.checked_add_assign(partial_size)?;
            total_notional = impact_size;

            break;
        }

        total_size.checked_add_assign(size)?;
        total_notional.checked_add_assign(order_notional)?;
    }

    if total_notional < impact_size {
        return Ok(None);
    }

    Ok(Some(total_notional.checked_div(total_size)?))
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {super::*, crate::Quantity, dango_primitives::StdResult};

    #[test]
    fn impact_price_empty_book() {
        let orders = std::iter::empty::<StdResult<(UsdPrice, Quantity)>>();
        let result = compute_impact_price(orders, UsdValue::new_int(10_000)).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn impact_price_insufficient_depth() {
        let orders = vec![Ok((UsdPrice::new_int(50_000), Quantity::new_int(1)))];
        let result = compute_impact_price(orders.into_iter(), UsdValue::new_int(100_000)).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn impact_price_exact_fill_single_order() {
        let orders = vec![Ok((UsdPrice::new_int(50_000), Quantity::new_int(2)))];
        let result = compute_impact_price(orders.into_iter(), UsdValue::new_int(100_000)).unwrap();
        assert_eq!(result, Some(UsdPrice::new_int(50_000)));
    }

    #[test]
    fn impact_price_partial_fill_last_order() {
        // Two orders: price=100, size=5 (notional=500) and price=110, size=10 (notional=1100)
        // Impact notional = 1000 → fill all of first (500) then 500/110 ≈ 4.545454 of second
        // total_size = 5 + 500/110 = 5 + 4.545454... = 9.545454...
        // VWAP = 1000 / 9.545454... = 104.761904...
        let orders = vec![
            Ok((UsdPrice::new_int(100), Quantity::new_int(5))),
            Ok((UsdPrice::new_int(110), Quantity::new_int(10))),
        ];
        let result = compute_impact_price(orders.into_iter(), UsdValue::new_int(1_000)).unwrap();
        let vwap = result.unwrap();
        assert!(vwap > UsdPrice::new_int(100));
        assert!(vwap < UsdPrice::new_int(110));
    }

    #[test]
    fn impact_price_multi_order_exact() {
        // Two orders that exactly fill: price=100, size=5 (500) + price=200, size=5 (1000)
        // total = 1500, need 1500, total_size=10 → VWAP = 1500/10 = 150
        let orders = vec![
            Ok((UsdPrice::new_int(100), Quantity::new_int(5))),
            Ok((UsdPrice::new_int(200), Quantity::new_int(5))),
        ];
        let result = compute_impact_price(orders.into_iter(), UsdValue::new_int(1_500)).unwrap();
        assert_eq!(result, Some(UsdPrice::new_int(150)));
    }
}
