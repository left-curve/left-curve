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
    std::collections::{BTreeMap, BTreeSet},
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
// ((base_denom, quote_denom), bucket_size, direction, price) => (amount_base, amount_quote)
pub const USER_DEPTHS: Map<
    ((&Denom, &Denom), Udec128_24, Direction, Udec128_24),
    (Udec128_6, Udec128_6),
> = Map::new("user_depth");

/// Liquidity depth from passive pool orders.
// (base_denom, quote_denom) => depths
pub const PASSIVE_DEPTHS: Map<(&Denom, &Denom), PassiveLiquidityDepths> = Map::new("passive_depth");

/// Stores the total trading volume in USD for each account address and timestamp.
pub const VOLUMES: Map<(&Addr, Timestamp), Udec128_6> = Map::new("volume");

/// Stores the total trading volume in USD for each username and timestamp.
pub const VOLUMES_BY_USER: Map<(&Username, Timestamp), Udec128_6> = Map::new("volume_by_user");

// bucket_size => direction => price => depth (total base asset)
pub type PassiveLiquidityDepths =
    BTreeMap<Udec128_24, BTreeMap<Direction, BTreeMap<Udec128_24, Udec128_6>>>;

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

/// Increase the liquidity depth when a user limit order is created.
pub fn increase_depths(
    storage: &mut dyn Storage,
    base_denom: &Denom,
    quote_denom: &Denom,
    direction: Direction,
    price: Udec128_24,
    amount_base: Udec128_6,
    amount_quote: Udec128_6,
    bucket_sizes: &BTreeSet<NonZero<Udec128_24>>,
) -> StdResult<()> {
    for bucket_size in bucket_sizes {
        let bucket = core::bucket(price, direction, *bucket_size)?;
        let key = ((base_denom, quote_denom), **bucket_size, direction, bucket);

        USER_DEPTHS.may_update(storage, key, |maybe_depths| -> StdResult<_> {
            let (depth_base, depth_quote) = maybe_depths.unwrap_or_default();

            let depth_base = depth_base.checked_add(amount_base)?;
            let depth_quote = depth_quote.checked_add(amount_quote)?;

            Ok((depth_base, depth_quote))
        })?;
    }

    Ok(())
}

/// Decrease the liquidity depth when a user limit order is canceled or fulfilled.
pub fn decrease_depths(
    storage: &mut dyn Storage,
    base_denom: &Denom,
    quote_denom: &Denom,
    direction: Direction,
    price: Udec128_24,
    amount_base: Udec128_6,
    amount_quote: Udec128_6,
    bucket_sizes: &BTreeSet<NonZero<Udec128_24>>,
) -> StdResult<()> {
    for bucket_size in bucket_sizes {
        let bucket = core::bucket(price, direction, *bucket_size)?;
        let key = ((base_denom, quote_denom), **bucket_size, direction, bucket);

        USER_DEPTHS.modify(storage, key, |(depth_base, depth_quote)| -> StdResult<_> {
            // Use saturating sub, in case underflows due to rounding.
            let depth_base = depth_base.saturating_sub(amount_base);
            let depth_quote = depth_quote.saturating_sub(amount_quote);

            // If any one is zero, we delete the entry.
            // TODO: can there be situations where only one is zero? maybe due to rounding?
            if depth_base.is_zero() || depth_quote.is_zero() {
                Ok(None)
            } else {
                Ok(Some((depth_base, depth_quote)))
            }
        })?;
    }

    Ok(())
}
