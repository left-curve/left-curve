use {
    dango_types::dex::{Order, OrderTrait},
    grug::{IsZero, Number, NumberConst, StdResult, Udec128_6, Udec128_24},
    std::cmp::Ordering,
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
    /// If a bid was popped out of the iterator but wasn't matched, it's
    /// returned here.
    pub unmatched_bid: Option<(Udec128_24, Order)>,
    /// If an ask was popped out of the iterator but wasn't matched, it's
    /// returned here.
    pub unmatched_ask: Option<(Udec128_24, Order)>,
    /// If the last bid that found a match was only partially matched, it's
    /// returned here.
    ///
    /// Since this order is only partially matched, it will remain in the book,
    /// and becomes the best available bid at the beginning of the next block.
    /// It is used to determine the resting order book state.
    pub last_partial_matched_bid: Option<(Udec128_24, Order)>,
    /// If the last ask that found a match was only partially matched, it's
    /// returned here.
    pub last_partial_matched_ask: Option<(Udec128_24, Order)>,
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
pub fn match_orders<B, A>(bid_iter: &mut B, ask_iter: &mut A) -> StdResult<MatchingOutcome>
where
    B: Iterator<Item = StdResult<(Udec128_24, Order)>>,
    A: Iterator<Item = StdResult<(Udec128_24, Order)>>,
{
    let mut bid = bid_iter.next().transpose()?;
    let mut bids = Vec::new();
    let mut bid_is_new = true;
    let mut bid_volume = Udec128_6::ZERO;
    let mut ask = ask_iter.next().transpose()?;
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
            bid = bid_iter.next().transpose()?;
            bid_is_new = true;
        } else {
            bid_is_new = false;
        }

        if ask_volume <= bid_volume {
            ask = ask_iter.next().transpose()?;
            ask_is_new = true;
        } else {
            ask_is_new = false;
        }
    }

    // The volume is the smaller between bid and ask volumes.
    let volume = bid_volume.min(ask_volume);

    // - If 0 < bid volume < ask volume, the last ask is partially filled.
    // - If 0 < ask volume < bid volume, the last bid is partially filled.
    // - If 0 < bid volume = ask volume, both last bid and ask are fully filled;
    //   return `None` for both.
    // - if 0 = bid volume = ask volume, no match found; return `None` for both.
    let (last_partial_matched_bid, last_partial_matched_ask) = if volume.is_non_zero() {
        match bid_volume.cmp(&ask_volume) {
            Ordering::Less => (None, asks.last().cloned()),
            Ordering::Greater => (bids.last().cloned(), None),
            Ordering::Equal => (None, None),
        }
    } else {
        (None, None)
    };

    Ok(MatchingOutcome {
        range,
        volume,
        bids,
        asks,
        unmatched_bid: bid,
        unmatched_ask: ask,
        last_partial_matched_bid,
        last_partial_matched_ask,
    })
}
