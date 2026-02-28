use {dango_types::UsdPrice, grug::MathResult};

/// When storing a bid order, we "invert" the price such that orders are sorted
/// according to price-time priority. Conversely, when reading orders from the
/// book, we need to "un-invert" the price. This function does both.
pub fn may_invert_price(price: UsdPrice, is_bid: bool) -> MathResult<UsdPrice> {
    if is_bid {
        UsdPrice::MAX.checked_sub(price)
    } else {
        Ok(price)
    }
}
