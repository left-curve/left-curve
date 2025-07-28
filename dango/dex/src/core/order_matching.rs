use {
    crate::{Order, OrderTrait},
    grug::{Number, NumberConst, StdResult, Udec128_6, Udec128_24},
    std::cmp::Ordering,
};

const HALF: Udec128_24 = Udec128_24::new_percent(50);

pub struct MatchingOutcome {
    /// The price at which the orders are to be filled at.
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
    let mut lower_created_at = None;
    let mut upper_created_at = None;

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
        lower_created_at = bid_order.created_at_block_height();
        upper_created_at = ask_order.created_at_block_height();

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

    // Select the price within the range.
    //
    // In a batch auction, we select the price that maximizes the trading volume
    // (measured in the base asset). In case a range of prices all results in
    // the maximum volume, we choose that of the more senior (older) order.
    // Orders from the passive liquidity pool are always considered order.
    // In case of a tie (same seniority), we choose the range's middle point.
    let clearing_price = range
        .map(
            |(lower, upper)| match (lower_created_at, upper_created_at) {
                (Some(lower_created_at), Some(upper_created_at)) => {
                    match lower_created_at.cmp(&upper_created_at) {
                        Ordering::Less => Ok(lower),
                        Ordering::Greater => Ok(upper),
                        Ordering::Equal => lower.checked_add(upper)?.checked_mul(HALF),
                    }
                },
                (None, Some(_)) => Ok(lower),
                (Some(_), None) => Ok(upper),
                (None, None) => {
                    unreachable!("passive pool orders shouldn't have a match within themselves");
                },
            },
        )
        .transpose()?;

    Ok(MatchingOutcome {
        clearing_price,
        volume: bid_volume.min(ask_volume),
        bids,
        asks,
    })
}
