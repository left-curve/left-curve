use {
    grug::{
        Addr, Map, Number, NumberConst, Order as IterationOrder, Prefixer, PrimaryKey, StdError,
        StdResult, Storage, Udec128, Uint128,
    },
    std::borrow::Cow,
};

pub type OrderId = u64;

// (direction, price) -> order
pub const ORDERS: Map<(Direction, Udec128, OrderId), Order> = Map::new("order");

#[grug::derive(Serde, Borsh)]
pub enum Direction {
    /// Give away the quote asset, get the base asset; a.k.a. a BUY order.
    Bid,
    /// Give away the base asset, get the quote asset; a.k.a. a SELL order.
    Ask,
}

impl PrimaryKey for Direction {
    type Output = Self;
    type Prefix = ();
    type Suffix = ();

    const KEY_ELEMS: u8 = 1;

    fn raw_keys(&self) -> Vec<Cow<[u8]>> {
        match self {
            Direction::Bid => vec![Cow::Borrowed(&[0])],
            Direction::Ask => vec![Cow::Borrowed(&[1])],
        }
    }

    fn from_slice(bytes: &[u8]) -> StdResult<Self::Output> {
        match bytes {
            [0] => Ok(Direction::Bid),
            [1] => Ok(Direction::Ask),
            _ => Err(StdError::deserialize::<Self::Output, _>(
                "key",
                format!("invalid order direction! must be 0|1"),
            )),
        }
    }
}

impl Prefixer for Direction {
    fn raw_prefixes(&self) -> Vec<Cow<[u8]>> {
        self.raw_keys()
    }
}

#[grug::derive(Serde, Borsh)]
pub struct Order {
    pub trader: Addr,
    // This is the amount of the base asset!
    pub amount: Uint128,
    pub remaining: Uint128,
}

#[derive(Debug, PartialEq)]
pub struct ClearOrderOutcome {
    pub range: Option<(Udec128, Udec128)>,
    pub volume: Uint128,
}

/// Execute matching orders, return the list of assets to be sent back to
/// traders whose orders have been filled.
///
/// Implemented according to:
/// <https://motokodefi.substack.com/p/uniform-price-call-auctions-a-better>
pub fn clear_orders(storage: &dyn Storage) -> StdResult<ClearOrderOutcome> {
    let mut bids =
        ORDERS
            .prefix(Direction::Bid)
            .range(storage, None, None, IterationOrder::Descending);

    let mut asks =
        ORDERS
            .prefix(Direction::Ask)
            .range(storage, None, None, IterationOrder::Ascending);

    let mut bid = bids.next().transpose()?;
    let mut bid_is_new = true;
    let mut bid_volume = Uint128::ZERO;
    let mut ask = asks.next().transpose()?;
    let mut ask_is_new = true;
    let mut ask_volume = Uint128::ZERO;
    let mut range = None;

    // Loop through the orders to find the execution price that maximizes volume
    // as measured in the base asset.
    loop {
        let Some(((bid_price, _), bid_order)) = &bid else {
            break;
        };

        let Some(((ask_price, _), ask_order)) = &ask else {
            break;
        };

        if bid_price < ask_price {
            break;
        }

        range = Some((*ask_price, *bid_price));

        if bid_is_new {
            bid_volume.checked_add_assign(bid_order.remaining)?;
        }

        if ask_is_new {
            ask_volume.checked_add_assign(ask_order.remaining)?;
        }

        if bid_volume <= ask_volume {
            bid = bids.next().transpose()?;
            bid_is_new = true;
        } else {
            bid_is_new = false;
        }

        if ask_volume <= bid_volume {
            ask = asks.next().transpose()?;
            ask_is_new = true;
        } else {
            ask_is_new = false;
        }
    }

    Ok(ClearOrderOutcome {
        range,
        volume: bid_volume.min(ask_volume),
    })
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {super::*, grug::MockStorage, test_case::test_case};

    // Test cases from:
    // https://motokodefi.substack.com/p/uniform-price-call-auctions-a-better
    #[test_case(
        [
            (Direction::Bid, Uint128::new(30), Uint128::new(10)),
            (Direction::Bid, Uint128::new(20), Uint128::new(10)),
            (Direction::Bid, Uint128::new(10), Uint128::new(10)),
            (Direction::Ask, Uint128::new(10), Uint128::new(10)),
            (Direction::Ask, Uint128::new(20), Uint128::new(10)),
            (Direction::Ask, Uint128::new(30), Uint128::new(10)),
        ],
        ClearOrderOutcome {
            range: Some((
                Uint128::new(20).checked_into_dec().unwrap(),
                Uint128::new(20).checked_into_dec().unwrap()
            )),
            volume: Uint128::new(20),
        };
        "example_one"
    )]
    #[test_case(
        [
            (Direction::Bid, Uint128::new(30), Uint128::new(10)),
            (Direction::Bid, Uint128::new(20), Uint128::new(10)),
            (Direction::Bid, Uint128::new(10), Uint128::new(10)),
            (Direction::Ask, Uint128::new(5), Uint128::new(10)),
            (Direction::Ask, Uint128::new(15), Uint128::new(10)),
            (Direction::Ask, Uint128::new(25), Uint128::new(10)),
        ],
        ClearOrderOutcome {
            range: Some((
                Uint128::new(15).checked_into_dec().unwrap(),
                Uint128::new(20).checked_into_dec().unwrap()
            )),
            volume: Uint128::new(20),
        };
        "example_two"
    )]
    #[test_case(
        [
            (Direction::Bid, Uint128::new(30), Uint128::new(10)),
            (Direction::Bid, Uint128::new(30), Uint128::new(5)),
            (Direction::Bid, Uint128::new(20), Uint128::new(10)),
            (Direction::Bid, Uint128::new(10), Uint128::new(10)),
            (Direction::Ask, Uint128::new(5), Uint128::new(10)),
            (Direction::Ask, Uint128::new(15), Uint128::new(10)),
            (Direction::Ask, Uint128::new(25), Uint128::new(10)),
        ],
        ClearOrderOutcome {
            range: Some((
                Uint128::new(15).checked_into_dec().unwrap(),
                Uint128::new(20).checked_into_dec().unwrap()
            )),
            volume: Uint128::new(20),
        };
        "example_three"
    )]
    #[test_case(
        [
            (Direction::Bid, Uint128::new(30), Uint128::new(20)),
            (Direction::Bid, Uint128::new(20), Uint128::new(10)),
            (Direction::Bid, Uint128::new(10), Uint128::new(10)),
            (Direction::Ask, Uint128::new(5), Uint128::new(10)),
            (Direction::Ask, Uint128::new(15), Uint128::new(10)),
            (Direction::Ask, Uint128::new(25), Uint128::new(10)),
        ],
        ClearOrderOutcome {
            range: Some((
                Uint128::new(15).checked_into_dec().unwrap(),
                Uint128::new(30).checked_into_dec().unwrap()
            )),
            volume: Uint128::new(20),
        };
        "example_four"
    )]
    fn clear_orders_works<const N: usize>(
        orders: [(Direction, Uint128, Uint128); N],
        expected: ClearOrderOutcome,
    ) {
        let mut storage = MockStorage::new();

        for (order_id, (direction, price, amount)) in orders.into_iter().enumerate() {
            ORDERS
                .save(
                    &mut storage,
                    (
                        direction,
                        price.checked_into_dec().unwrap(),
                        order_id as u64,
                    ),
                    &Order {
                        trader: Addr::mock(0),
                        amount,
                        remaining: amount,
                    },
                )
                .unwrap();
        }

        let outcome = clear_orders(&storage).unwrap();
        assert_eq!(outcome, expected);
    }
}
