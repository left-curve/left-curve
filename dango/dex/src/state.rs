use {
    crate::core,
    dango_types::{
        account_factory::Username,
        dex::{Direction, LimitOrder, MarketOrder, OrderId, PairParams, RestingOrderBookState},
    },
    grug::{
        Addr, CoinPair, Counter, Denom, IndexedMap, IsZero, Item, Map, MultiIndex, NonZero, Number,
        NumberConst, StdResult, Storage, Timestamp, Udec128_6, Udec128_24, Uint64, UniqueIndex,
    },
    std::collections::BTreeSet,
};

pub const PAUSED: Item<bool> = Item::new("paused");

// (base_denom, quote_denom) => params
pub const PAIRS: Map<(&Denom, &Denom), PairParams> = Map::new("pair");

// (base_denom, quote_denom) => coin_pair
pub const RESERVES: Map<(&Denom, &Denom), CoinPair> = Map::new("reserve");

pub const RESTING_ORDER_BOOK: Map<(&Denom, &Denom), RestingOrderBookState> = Map::new("resting");

pub const NEXT_ORDER_ID: Counter<OrderId> = Counter::new("order_id", Uint64::ONE, Uint64::ONE);

pub const MARKET_ORDERS: Map<(Addr, OrderId), (OrderKey, MarketOrder)> = Map::new("market");

pub const LIMIT_ORDERS: IndexedMap<OrderKey, LimitOrder, LimitOrderIndex> =
    IndexedMap::new("order", LimitOrderIndex {
        order_id: UniqueIndex::new(|(_, _, _, order_id), _| *order_id, "order", "order__id"),
        user: MultiIndex::new(|_, order| order.user, "order", "order__user"),
    });

/// Liquidity depth from user orders.
// ((base_denom, quote_denom), bucket_size, direction, price)
pub const USER_DEPTHS: Map<((&Denom, &Denom), Udec128_24, Direction, Udec128_24), Udec128_6> =
    Map::new("user_depth");

/// Liquidity depth from passive pool orders.
// ((base_denom, quote_denom), bucket_size, direction, price)
pub const PASSIVE_DEPTHS: Map<((&Denom, &Denom), Udec128_24, Direction, Udec128_24), Udec128_6> =
    Map::new("passive_depth");

/// Stores the total trading volume in USD for each account address and timestamp.
pub const VOLUMES: Map<(&Addr, Timestamp), Udec128_6> = Map::new("volume");

/// Stores the total trading volume in USD for each username and timestamp.
pub const VOLUMES_BY_USER: Map<(&Username, Timestamp), Udec128_6> = Map::new("volume_by_user");

/// Storage key for orders, both limit and market.
///
/// - For limit orders, the `price` is the limit price.
/// - For market orders, it is calculated based on the best price available in
///   the resting order book and the order's maximum slippage.
///
/// ```plain
/// ((base_denom, quote_denom), direction, price, order_id)
/// ```
pub type OrderKey = ((Denom, Denom), Direction, Udec128_24, OrderId);

#[grug::index_list(OrderKey, LimitOrder)]
pub struct LimitOrderIndex<'a> {
    pub order_id: UniqueIndex<'a, OrderKey, OrderId, LimitOrder>,
    pub user: MultiIndex<'a, OrderKey, Addr, LimitOrder>,
}

pub fn increase_depths(
    map: &Map<((&Denom, &Denom), Udec128_24, Direction, Udec128_24), Udec128_6>,
    storage: &mut dyn Storage,
    base_denom: &Denom,
    quote_denom: &Denom,
    direction: Direction,
    price: Udec128_24,
    amount: Udec128_6,
    bucket_sizes: &BTreeSet<NonZero<Udec128_24>>,
) -> StdResult<()> {
    for bucket_size in bucket_sizes {
        let bucket = core::bucket(price, direction, *bucket_size)?;
        let key = ((base_denom, quote_denom), **bucket_size, direction, bucket);

        map.may_update(storage, key, |maybe_depth| -> StdResult<_> {
            let depth = maybe_depth.unwrap_or(Udec128_6::ZERO);
            Ok(depth.checked_add(amount)?)
        })?;
    }

    Ok(())
}

pub fn decrease_depths(
    map: &Map<((&Denom, &Denom), Udec128_24, Direction, Udec128_24), Udec128_6>,
    storage: &mut dyn Storage,
    base_denom: &Denom,
    quote_denom: &Denom,
    direction: Direction,
    price: Udec128_24,
    filled: Udec128_6,
    bucket_sizes: &BTreeSet<NonZero<Udec128_24>>,
) -> StdResult<()> {
    for bucket_size in bucket_sizes {
        let bucket = core::bucket(price, direction, *bucket_size)?;
        let key = ((base_denom, quote_denom), **bucket_size, direction, bucket);

        map.modify(storage, key, |depth| -> StdResult<_> {
            let depth = depth.checked_sub(filled)?;
            if depth.is_zero() {
                Ok(None)
            } else {
                Ok(Some(depth))
            }
        })?;
    }

    Ok(())
}
