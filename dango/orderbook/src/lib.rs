use grug::{
    Addr, Map, Number, NumberConst, Order as IterationOrder, PrimaryKey, RawKey, StdError,
    StdResult, Storage, Udec128, Uint128,
};

pub type OrderId = u64;

pub const ORDERS: Map<OrderKey, Order> = Map::new("order");

#[grug::derive(Serde, Borsh)]
#[derive(Copy)]
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

    fn raw_keys(&self) -> Vec<RawKey> {
        match self {
            Direction::Bid => vec![RawKey::Fixed8([0])],
            Direction::Ask => vec![RawKey::Fixed8([1])],
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrderKey {
    pub direction: Direction,
    pub price: Udec128,
    pub order_id: OrderId,
}

impl PrimaryKey for OrderKey {
    type Output = (Direction, Udec128, OrderId);
    type Prefix = Direction;
    type Suffix = (Udec128, OrderId);

    const KEY_ELEMS: u8 = 3;

    fn raw_keys(&self) -> Vec<RawKey> {
        let mut keys = self.direction.raw_keys();
        keys.extend(self.price.raw_keys());
        // For BUY orders, we use the bitwise reverse of `order_id` (which equals
        // `u64::MAX - order_id` numerically) such that older orders are filled.
        // first. This follows the _price-time priority_ rule.
        //
        // Note that this assumes `order_id` never exceeds `u64::MAX / 2`, which
        // is a safe assumption. Even if we accept 1 million orders per second,
        // it would take 5.4e+24 years to reach `u64::MAX / 2` which is about
        // 400 trillion times the age of the universe. The Sun will become a red
        // giant and devour Earth in 5 billion years so by then we're all gone.
        keys.push(RawKey::Fixed64(
            match self.direction {
                Direction::Bid => !self.order_id,
                Direction::Ask => self.order_id,
            }
            .to_be_bytes(),
        ));
        keys
    }

    fn from_slice(bytes: &[u8]) -> StdResult<Self::Output> {
        let (direction, price, order_id) = <(Direction, Udec128, OrderId)>::from_slice(bytes)?;
        match direction {
            Direction::Bid => Ok((direction, price, !order_id)),
            Direction::Ask => Ok((direction, price, order_id)),
        }
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
                    OrderKey {
                        direction,
                        price: price.checked_into_dec().unwrap(),
                        order_id: order_id as OrderId,
                    },
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
