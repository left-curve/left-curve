use {
    crate::{Order, OrderTrait},
    grug::{Number, NumberConst, StdResult, Udec128_6, Udec128_24},
};

const HALF: Udec128_24 = Udec128_24::new_percent(50);

pub struct MatchingOutcome {
    /// The single clearing price for all matched limit orders determined by the
    /// batch auction.
    pub clearing_price: Option<Udec128_24>,
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
pub fn match_limit_orders<B, A>(mut bid_iter: B, mut ask_iter: A) -> StdResult<MatchingOutcome>
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

    // Find the current mid price, if order exists on both sides of the book.
    // If there are only orders on one side, take the price of this side.
    // If there is no order, then nothing to do, return.
    let mid_price = match (bid, ask) {
        (Some((bid_price, _)), Some((ask_price, _))) => {
            Some(bid_price.checked_add(ask_price)?.checked_mul(HALF)?)
        },
        (Some((bid_price, _)), None) => Some(bid_price),
        (None, Some((ask_price, _))) => Some(ask_price),
        (None, None) => {
            return Ok(MatchingOutcome {
                clearing_price: None,
                volume: Udec128_6::ZERO,
                bids,
                asks,
            });
        },
    };

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

    // Select the clearing price.
    let clearing_price = if let Some((ask_price, bid_price)) = range {
        if let Some(mid_price) = mid_price {
            if bid_price <= mid_price {
                Some(bid_price)
            } else if ask_price >= mid_price {
                Some(ask_price)
            } else {
                Some(mid_price)
            }
        } else {
            Some(bid_price.checked_add(ask_price)?.checked_mul(HALF)?)
        }
    } else {
        None
    };

    Ok(MatchingOutcome {
        clearing_price,
        volume: bid_volume.min(ask_volume),
        bids,
        asks,
    })
}
