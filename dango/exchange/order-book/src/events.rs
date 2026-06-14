use {
    crate::{ClientOrderId, Dimensionless, OrderId, PairId, Quantity, TriggerDirection, UsdPrice},
    dango_primitives::Addr,
};

/// Event indicating an order have been inserted into the order book.
#[dango_primitives::event("order_persisted")]
#[dango_primitives::derive(Serde)]
pub struct OrderPersisted {
    pub order_id: OrderId,
    pub pair_id: PairId,
    pub user: Addr,
    pub limit_price: UsdPrice,
    pub size: Quantity,

    /// Caller-assigned id from the originally-submitted order, or `None`
    /// if the order was submitted without one.
    pub client_order_id: Option<ClientOrderId>,
}

/// Event indicating a resting order's size was reduced in place (not removed).
///
/// Emitted by dynamic re-sizing of reduce-only orders: when a user's position
/// shrinks, their resting reduce-only orders are clamped so that their absolute
/// sizes sum to no more than the new position. An order whose clamped size is
/// still non-zero is rewritten with the smaller size rather than removed (a
/// clamp to zero removes it instead, emitting `OrderRemoved`).
#[dango_primitives::event("order_resized")]
#[dango_primitives::derive(Serde)]
pub struct OrderResized {
    pub order_id: OrderId,
    pub pair_id: PairId,
    pub user: Addr,

    /// Signed size before the re-size.
    pub old_size: Quantity,

    /// Signed size after the re-size. Carries the same sign as `old_size`, with
    /// a strictly smaller absolute value.
    pub new_size: Quantity,

    /// Caller-assigned id from the originally-submitted order, or `None`
    /// if the order was submitted without one.
    pub client_order_id: Option<ClientOrderId>,
}

/// Event indicating an order has been removed from the order book.
#[dango_primitives::event("order_removed")]
#[dango_primitives::derive(Serde)]
pub struct OrderRemoved {
    pub order_id: OrderId,
    pub pair_id: PairId,
    pub user: Addr,
    pub reason: ReasonForOrderRemoval,

    /// Caller-assigned id from the originally-submitted order, or `None`
    /// if the order was submitted without one.
    pub client_order_id: Option<ClientOrderId>,
}

/// Event indicating a conditional (TP/SL) order has been placed.
#[dango_primitives::event("conditional_order_placed")]
#[dango_primitives::derive(Serde)]
pub struct ConditionalOrderPlaced {
    pub pair_id: PairId,
    pub user: Addr,
    pub trigger_price: UsdPrice,
    pub trigger_direction: TriggerDirection,
    pub size: Option<Quantity>,
    pub max_slippage: Dimensionless,
}

/// Event indicating a conditional order was triggered by an oracle price move.
#[dango_primitives::event("conditional_order_triggered")]
#[dango_primitives::derive(Serde)]
pub struct ConditionalOrderTriggered {
    pub pair_id: PairId,
    pub user: Addr,
    pub trigger_price: UsdPrice,
    pub trigger_direction: TriggerDirection,
    pub oracle_price: UsdPrice,
}

/// Event indicating a conditional order was removed.
#[dango_primitives::event("conditional_order_removed")]
#[dango_primitives::derive(Serde)]
pub struct ConditionalOrderRemoved {
    pub pair_id: PairId,
    pub user: Addr,
    pub trigger_direction: TriggerDirection,
    pub reason: ReasonForOrderRemoval,
}

#[dango_primitives::derive(Serde)]
#[derive(Copy)]
pub enum ReasonForOrderRemoval {
    /// The order was fully filled.
    Filled,

    /// The user voluntarily canceled the order.
    Canceled,

    /// In case of conditional (TP/SL) orders, the position was closed or flipped.
    PositionClosed,

    /// The user submitted an order on the other side of the order book whose
    /// price crossed this order's. Following the principle of self-trade prevention,
    /// this order was canceled.
    SelfTradePrevention,

    /// The user was liquidated.
    Liquidated,

    /// The user was hit by auto-deleveraging (ADL).
    Deleveraged,

    /// A resting reduce-only order was cancelled by dynamic re-sizing because
    /// the user's position changed: it closed or flipped (the order is now on
    /// the wrong side), or it shrank enough that this order — the worst by
    /// price-time priority among the user's reduce-only orders — had no
    /// remaining position left to close. Distinct from `PositionClosed`, which
    /// is specific to conditional (TP/SL) orders.
    ReduceOnlyResized,

    /// The conditional order was triggered but could not fill within the
    /// user's max_slippage tolerance (insufficient book liquidity).
    SlippageExceeded,

    /// The resting order's price fell outside the pair's
    /// `max_limit_price_deviation` band at the time it was about to match
    /// (i.e. the oracle moved after the order was placed). The matching
    /// engine cancels such stale orders and walks deeper in the book.
    PriceBandViolation,

    /// A conditional (TP/SL) order was triggered but its stored
    /// `max_slippage` now exceeds the pair's `max_market_slippage` cap —
    /// governance tightened the cap between the order's submission and
    /// its trigger. The order is cancelled rather than submitted. Distinct
    /// from `SlippageExceeded` so the event stream can tell a policy
    /// tightening apart from a liquidity shortfall.
    SlippageCapTightened,
}
