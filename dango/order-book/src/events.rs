use {
    crate::{ClientOrderId, Dimensionless, OrderId, PairId, Quantity, TriggerDirection, UsdPrice},
    grug::Addr,
};

/// Event indicating an order have been inserted into the order book.
#[grug::event("order_persisted")]
#[grug::derive(Serde)]
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

/// Event indicating an order has been removed from the order book.
#[grug::event("order_removed")]
#[grug::derive(Serde)]
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
#[grug::event("conditional_order_placed")]
#[grug::derive(Serde)]
pub struct ConditionalOrderPlaced {
    pub pair_id: PairId,
    pub user: Addr,
    pub trigger_price: UsdPrice,
    pub trigger_direction: TriggerDirection,
    pub size: Option<Quantity>,
    pub max_slippage: Dimensionless,
}

/// Event indicating a conditional order was triggered by an oracle price move.
#[grug::event("conditional_order_triggered")]
#[grug::derive(Serde)]
pub struct ConditionalOrderTriggered {
    pub pair_id: PairId,
    pub user: Addr,
    pub trigger_price: UsdPrice,
    pub trigger_direction: TriggerDirection,
    pub oracle_price: UsdPrice,
}

/// Event indicating a conditional order was removed.
#[grug::event("conditional_order_removed")]
#[grug::derive(Serde)]
pub struct ConditionalOrderRemoved {
    pub pair_id: PairId,
    pub user: Addr,
    pub trigger_direction: TriggerDirection,
    pub reason: ReasonForOrderRemoval,
}

#[grug::derive(Serde)]
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
