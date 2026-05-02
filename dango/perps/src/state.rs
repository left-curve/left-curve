use {
    dango_order_book::{Dimensionless, Quantity, UsdPrice, UsdValue},
    dango_types::{
        account_factory::UserIndex,
        perps::{
            ClientOrderId, CommissionRate, ConditionalOrderId, FeeShareRatio, FillId, LimitOrder,
            OrderId, PairId, PairParam, PairState, Param, Referee, RefereeStats, Referrer, State,
            TriggerDirection, UserReferralData, UserState, VaultSnapshot,
        },
    },
    grug::{Addr, IndexedMap, Item, Map, MultiIndex, Set, Timestamp, UniqueIndex},
    std::collections::BTreeSet,
};

// --------------------------------- constants ---------------------------------

pub const NEXT_ORDER_ID: Item<OrderId> = Item::new("next_order_id");

pub const NEXT_FILL_ID: Item<FillId> = Item::new("next_fill_id");

pub const LAST_VAULT_ORDERS_UPDATE: Item<u64> = Item::new("last_vault_orders_update");

pub const PARAM: Item<Param> = Item::new("param");

pub const STATE: Item<State> = Item::new("state");

pub const PAIR_IDS: Item<BTreeSet<PairId>> = Item::new("pair_ids");

pub const PAIR_PARAMS: Map<&PairId, PairParam> = Map::new("pair_param");

pub const PAIR_STATES: Map<&PairId, PairState> = Map::new("pair_state");

pub const USER_STATES: IndexedMap<Addr, UserState, UserStateIndexes> =
    IndexedMap::new("us", UserStateIndexes::new("us", "us__unlock", "us__cond"));

/// For a given trading pair, users who have _long_ positions in this pair,
/// indexed by their entry prices.
///
/// Used during auto-deleveraging (ADL) to find the most profitable positions.
pub const LONGS: Set<(PairId, UsdPrice, Addr)> = Set::new("long");

/// For a given trading pair, users who have _short_ positions in this pair,
/// indexed by their entry prices.
///
/// Used during auto-deleveraging (ADL) to find the most profitable positions.
pub const SHORTS: Set<(PairId, UsdPrice, Addr)> = Set::new("short");

/// Buy orders.
pub const BIDS: IndexedMap<OrderKey, LimitOrder, OrderIndexes> = IndexedMap::new(
    "bid",
    OrderIndexes::new("bid", "bid__id", "bid__user", "bid__cid"),
);

/// Sell orders.
pub const ASKS: IndexedMap<OrderKey, LimitOrder, OrderIndexes> = IndexedMap::new(
    "ask",
    OrderIndexes::new("ask", "ask__id", "ask__user", "ask__cid"),
);

/// Liquidity depths of the order book.
pub const DEPTHS: Map<DepthKey, (Quantity, UsdValue)> = Map::new("depth");

/// Cumulative trading volume per user, bucketed by day.
/// Key: (user, day_timestamp). Value: lifetime cumulative USD notional.
pub const VOLUMES: Map<(Addr, Timestamp), UsdValue> = Map::new("vol");

/// Daily snapshots of the market-making vault's `(equity, share_supply)`.
/// Keys are block timestamps rounded down to the start of the day. Used
/// off-chain to compute the vault's historical share-price curve / APR.
pub const VAULT_SNAPSHOTS: Map<Timestamp, VaultSnapshot> = Map::new("vault_snap");

/// Address --> (maker_fee_rate, taker_fee_rate)
pub const FEE_RATE_OVERRIDES: Map<Addr, (Dimensionless, Dimensionless)> = Map::new("fr_override");

// --------------------------------- referral ----------------------------------

/// Maps a referee to their referrer. Immutable once set, except that the chain
/// owner can override an existing mapping via `SetReferral`.
pub const REFEREE_TO_REFERRER: Map<UserIndex, UserIndex> = Map::new("ref_r");

/// Maps a referrer to their fee share ratio.
pub const FEE_SHARE_RATIO: Map<UserIndex, FeeShareRatio> = Map::new("ref_sr");

/// Per-user commission rate override. If set, this value is used instead of
/// the volume-based tier calculation.
pub const COMMISSION_RATE_OVERRIDES: Map<UserIndex, CommissionRate> = Map::new("ref_cr_override");

/// Cumulative referral data per user, bucketed by day.
pub const USER_REFERRAL_DATA: Map<(UserIndex, Timestamp), UserReferralData> = Map::new("ref_data");

/// Per-referee statistics from the referrer's perspective, with multi-indexes
/// for sorted queries by registration date, volume, and commission.
pub const REFERRER_TO_REFEREE_STATISTICS: IndexedMap<
    (Referrer, Referee),
    RefereeStats,
    ReferrerStatisticsIndex,
> = IndexedMap::new(
    "ref_stat",
    ReferrerStatisticsIndex::new(
        "ref_stat",
        "ref_stat__registered_at",
        "ref_stat__volume",
        "ref_stat__commission",
    ),
);

// ----------------------------------- types -----------------------------------

pub type OrderKey = (PairId, UsdPrice, OrderId);

#[grug::index_list(OrderKey, LimitOrder)]
pub struct OrderIndexes<'a> {
    pub order_id: UniqueIndex<'a, OrderKey, OrderId, LimitOrder>,
    pub user: MultiIndex<'a, OrderKey, Addr, LimitOrder>,
    /// Lets a trader cancel an order in the same block it was submitted, by
    /// the caller-assigned `client_order_id`. The index function returns at
    /// most one key — empty `Vec` for orders submitted without a
    /// `client_order_id`. Uniqueness is per-sender, enforced by
    /// `UniqueIndex` (returns `StdError::duplicate_data` on collision).
    pub client_order_id: UniqueIndex<'a, OrderKey, (Addr, ClientOrderId), LimitOrder>,
}

impl OrderIndexes<'static> {
    pub const fn new(
        pk_namespace: &'static str,
        order_id_namespace: &'static str,
        user_namespace: &'static str,
        client_order_id_namespace: &'static str,
    ) -> Self {
        OrderIndexes {
            order_id: UniqueIndex::new(
                |(_, _, order_id), _| *order_id,
                pk_namespace,
                order_id_namespace,
            ),
            user: MultiIndex::new(|_, order| order.user, pk_namespace, user_namespace),
            client_order_id: UniqueIndex::new2(
                |_, order| match order.client_order_id {
                    Some(cid) => vec![(order.user, cid)],
                    None => Vec::new(),
                },
                pk_namespace,
                client_order_id_namespace,
            ),
        }
    }
}

#[grug::index_list(Addr, UserState)]
pub struct UserStateIndexes<'a> {
    /// If the user state has one or more pending unlocks, the earliest ending
    /// time of those unlocks; otherwise, `Timestamp::MAX`.
    pub earliest_unlock_end_time: MultiIndex<'a, Addr, Timestamp, UserState>,

    /// Conditional orders across a user's positions.
    /// For BELOW orders, the trigger price is inverted, so that ascending
    /// iteration visits the highest prices first.
    pub conditional_orders:
        MultiIndex<'a, Addr, (PairId, TriggerDirection, UsdPrice, ConditionalOrderId), UserState>,
}

impl UserStateIndexes<'static> {
    pub const fn new(
        pk_namespace: &'static str,
        unlock_namespace: &'static str,
        cond_namespace: &'static str,
    ) -> Self {
        UserStateIndexes {
            earliest_unlock_end_time: MultiIndex::new(
                |_, user_state| {
                    user_state
                        .unlocks
                        .front()
                        .map(|unlock| unlock.end_time)
                        .unwrap_or(Timestamp::MAX)
                },
                pk_namespace,
                unlock_namespace,
            ),
            conditional_orders: MultiIndex::new2(
                |_, user_state| {
                    let mut keys = Vec::new();
                    for (pair_id, position) in &user_state.positions {
                        if let Some(order) = &position.conditional_order_above {
                            keys.push((
                                pair_id.clone(),
                                TriggerDirection::Above,
                                order.trigger_price,
                                order.order_id,
                            ));
                        }
                        if let Some(order) = &position.conditional_order_below {
                            keys.push((
                                pair_id.clone(),
                                TriggerDirection::Below,
                                !order.trigger_price,
                                order.order_id,
                            ));
                        }
                    }
                    keys
                },
                pk_namespace,
                cond_namespace,
            ),
        }
    }
}

#[grug::index_list((Referrer, Referee), RefereeStats)]
pub struct ReferrerStatisticsIndex<'a> {
    pub registered_at: MultiIndex<'a, (Referrer, Referee), (Referrer, Timestamp), RefereeStats>,
    pub volume: MultiIndex<'a, (Referrer, Referee), (Referrer, UsdValue), RefereeStats>,
    pub commission: MultiIndex<'a, (Referrer, Referee), (Referrer, UsdValue), RefereeStats>,
}

impl ReferrerStatisticsIndex<'static> {
    pub const fn new(
        pk_namespace: &'static str,
        registered_at_namespace: &'static str,
        volume_namespace: &'static str,
        commission_namespace: &'static str,
    ) -> Self {
        ReferrerStatisticsIndex {
            registered_at: MultiIndex::new(
                |(referrer, _), data| (*referrer, data.registered_at),
                pk_namespace,
                registered_at_namespace,
            ),
            volume: MultiIndex::new(
                |(referrer, _), data| (*referrer, data.volume),
                pk_namespace,
                volume_namespace,
            ),
            commission: MultiIndex::new(
                |(referrer, _), data| (*referrer, data.commission_earned),
                pk_namespace,
                commission_namespace,
            ),
        }
    }
}

pub type DepthKey<'a> = (&'a PairId, UsdPrice, bool, UsdPrice);
