use {
    dango_types::dex::{Order, OrderTrait},
    grug::{Number, NumberConst, StdResult, Udec128_6, Udec128_24},
};

pub struct MatchingOutcome {
    /// The range of prices that achieve the biggest trading volume.
    /// `None` if no match is found.
    ///
    /// All prices in this range achieve the same volume. It's up to the caller
    /// to decide which price to use: the lowest, the highest, or the midpoint.
    pub range: Option<(Udec128_24, Udec128_24)>,
    /// The amount of trading volume, measured as the amount of the base asset.
    pub volume: Udec128_6,
    /// The BUY orders that have found a match.
    pub bids: Vec<(Udec128_24, Order)>,
    /// The SELL orders that have found a match.
    pub asks: Vec<(Udec128_24, Order)>,
}

/// Given the standing BUY and SELL orders in the book, find range of prices
/// that maximizes the trading volume.
///
/// ## Inputs:
///
/// - `bid_iter`: An iterator over the BUY orders in the book. This should
///   follow the **price-time priority**, meaning it should return the order
///   with the best price (in the case of BUY orders, the highest price) first;
///   for orders the same price, the oldest one first.
/// - `ask_iter`: An iterator over the SELL orders in the book that similarly
///   follows the price-time priority.
pub fn match_limit_orders<B, A>(bid_iter: &mut B, ask_iter: &mut A) -> StdResult<MatchingOutcome>
where
    B: Iterator<Item = (Udec128_24, Order)>,
    A: Iterator<Item = (Udec128_24, Order)>,
{
    let mut bid = bid_iter.next();
    let mut bids = Vec::new();
    let mut bid_is_new = true;
    let mut bid_volume = Udec128_6::ZERO;
    let mut ask = ask_iter.next();
    let mut asks = Vec::new();
    let mut ask_is_new = true;
    let mut ask_volume = Udec128_6::ZERO;
    let mut range = None;

    loop {
        let Some((bid_price, bid_order)) = bid else {
            break;
        };

        let Some((ask_price, ask_order)) = ask else {
            break;
        };

        if bid_price < ask_price {
            break;
        }

        range = Some((ask_price, bid_price));

        if bid_is_new {
            bids.push((bid_price, bid_order));
            bid_volume.checked_add_assign(*bid_order.remaining())?;
        }

        if ask_is_new {
            asks.push((ask_price, ask_order));
            ask_volume.checked_add_assign(*ask_order.remaining())?;
        }

        if bid_volume <= ask_volume {
            bid = bid_iter.next();
            bid_is_new = true;
        } else {
            bid_is_new = false;
        }

        if ask_volume <= bid_volume {
            ask = ask_iter.next();
            ask_is_new = true;
        } else {
            ask_is_new = false;
        }
    }

    Ok(MatchingOutcome {
        range,
        volume: bid_volume.min(ask_volume),
        bids,
        asks,
    })
}
