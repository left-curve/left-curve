use crate::UsdPrice;

/// When storing a bid order, we "invert" the price such that orders are sorted
/// according to price-time priority. Conversely, when reading orders from the
/// book, we need to "un-invert" the price. This function does both.
///
/// Uses bitwise NOT (`!price`) which reverses ordering without overflow risk
/// and is its own inverse: `!(!x) == x`.
pub fn may_invert_price(price: UsdPrice, is_bid: bool) -> UsdPrice {
    if is_bid {
        !price
    } else {
        price
    }
}
