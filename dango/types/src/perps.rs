use {
    crate::{
        Dimensionless, FundingPerUnit, FundingRate, FundingVelocity, Quantity, UsdPrice, UsdValue,
    },
    grug::{Addr, Denom, Duration, Part, Timestamp, Uint64, Uint128},
    std::{
        collections::{BTreeMap, BTreeSet, VecDeque},
        sync::LazyLock,
    },
};

// --------------------------------- Constants ---------------------------------

/// Denomination of the asset used to settle perpetual futures contracts.
pub use crate::constants::usdc as settlement_currency;

/// Namespace for tokens minted by the perps contract.
pub static NAMESPACE: LazyLock<Part> = LazyLock::new(|| Part::new_unchecked("perps"));

/// Sub-denomination of the vault share token.
pub static SUBDENOM: LazyLock<Part> = LazyLock::new(|| Part::new_unchecked("vault"));

/// Full denom of the vault share token.
pub static DENOM: LazyLock<Denom> = LazyLock::new(|| Denom::new_unchecked(["perps", "vault"]));

// ----------------------------------- Types -----------------------------------

/// Identifier of a trading pair. It should be a string that looks like e.g. "perp/btcusd".
pub type PairId = Denom;

/// Identifies a resting limit order.
pub type OrderId = Uint64;

#[grug::derive(Serde)]
#[derive(Copy)]
pub enum OrderKind {
    /// Trade at the best available prices in the order book, optionally
    /// with a slippage tolerance relative to the oracle price.
    ///
    /// If the order cannot be fully filled, the unfilled portion is
    /// canceled (immediate-or-cancel behavior).
    Market { max_slippage: Dimensionless },

    /// Trade at the specified limit price.
    ///
    /// When `post_only` is true, the order is rejected if it would
    /// immediately match against a resting order on the opposite side.
    /// This guarantees the submitter becomes a maker.
    Limit {
        limit_price: UsdPrice,
        post_only: bool,
    },
}

impl OrderKind {
    /// If this is a post-only limit order, return the limit price.
    /// Otherwise, return `None`.
    pub fn post_only_price(self) -> Option<UsdPrice> {
        match self {
            OrderKind::Limit {
                limit_price,
                post_only: true,
            } => Some(limit_price),
            _ => None,
        }
    }
}

/// Global parameters that concerns the counterparty vault and all trading pairs.
#[grug::derive(Serde, Borsh)]
#[derive(Default)]
pub struct Param {
    /// Once a request to withdraw liquidity from the counterparty vault has been
    /// submitted, the waiting time that must elapsed before the funds are released
    /// to the liquidity provider.
    pub vault_cooldown_period: Duration,

    /// Maximum number of unlock requests a single user may have.
    pub max_unlocks: usize,

    /// Maximum number of resting limit order a single user may have across all
    /// trading pairs.
    pub max_open_orders: usize,

    /// Fee charged to makers (limit orders that rest on the book) as a fraction
    /// of the fill's notional value, deducted from the user's margin and
    /// transferred to the vault on every fill.
    pub maker_fee_rate: Dimensionless,

    /// Fee charged to takers (market orders and crossing limit orders) as a
    /// fraction of the fill's notional value, deducted from the user's margin
    /// and transferred to the vault on every fill.
    pub taker_fee_rate: Dimensionless,

    /// Fee paid to the vault as a fraction of the total notional value of
    /// positions being liquidated, capped at the user's remaining margin
    /// after position closure.
    ///
    /// fee = min(
    ///   ceil(|position_size| * oracle_price * liquidation_fee_rate / settlement_currency_price),
    ///   user_remaining_margin
    /// )
    pub liquidation_fee_rate: Dimensionless,

    /// Set of addresses authorized to call `Deleverage`.
    pub adl_operators: BTreeSet<Addr>,
}

/// Parameters that apply to an individual trading pair.
#[grug::derive(Serde, Borsh)]
#[derive(Default)]
pub struct PairParam {
    /// A scaling factor that determines how greatly an imbalance in long/short
    /// open interests (the "skew") should affect the funding rate. The greater
    /// the value of the scaling factor, the less the effect. Used only for the
    /// funding fee mechanism (not for pricing, which is determined by the order
    /// book).
    pub skew_scale: Quantity,

    /// The maximum allowed open interest for both long and short.
    /// I.e. the following must be satisfied:
    ///
    /// pair_state.long_oi <= max_abs_oi && pair_state.short_oi <= max_abs_oi
    ///
    /// This constraint does not apply to reduce-only orders.
    pub max_abs_oi: Quantity,

    /// Minimum price increment for limit orders in this pair. All limit order
    /// prices must be an integer multiple of `tick_size`.
    pub tick_size: UsdPrice,

    /// Half the bid-ask spread the vault quotes around the oracle price. The
    /// vault places bids at `oracle_price * (1 - vault_half_spread)` and asks
    /// at `oracle_price * (1 + vault_half_spread)`.
    pub vault_half_spread: Dimensionless,

    /// Maximum notional size (in quote currency) of the vault's resting orders
    /// on each side of the book. Limits the vault's exposure per pair.
    pub vault_max_quote_size: Quantity,

    /// Maximum absolute funding rate, as a fraction per day.
    ///
    /// That is, the daily funding rate is clamped to the range
    /// [-max_abs_funding_rate, max_abs_funding_rate].
    ///
    /// This prevents runaway rates from causing cascading liquidations and bad
    /// debt spirals during prolonged skew.
    pub max_abs_funding_rate: FundingRate,

    /// Maximum rate the funding rate may change, as a fraction per day.
    ///
    /// When |skew| = skew_scale, the funding rate changes by this much per day.
    /// When skew == 0, the rate drifts back toward zero at this speed.
    pub max_funding_velocity: FundingVelocity,

    /// Minimum notional value for an order. Reduce-only orders are exempt.
    /// Prevents dust orders from cluttering the order book.
    #[serde(alias = "min_opening_size")]
    pub min_order_size: UsdValue,

    /// Margin requirement when opening or increasing a position in this trading
    /// pair. E.g. 5% indicates a 1 / 5% = 20x maximum leverage.
    ///
    /// initial_margin = |position_size| * oracle_price * initial_margin_ratio
    pub initial_margin_ratio: Dimensionless,

    /// Margin requirement for maintaining a position in this trading pair.
    ///
    /// Must be strictly less than `initial_margin_ratio`.
    ///
    /// When a user's equity falls below the sum of maintenance margins across
    /// all his positions, the user becomes eligible for liquidations.
    ///
    /// maintenance_margin = |position_size| * oracle_price * maintenance_margin_ratio
    pub maintenance_margin_ratio: Dimensionless,
}

impl PairParam {
    /// Build a `PairParam` with the funding-relevant fields varied;
    /// all other fields use inert defaults. Intended for tests.
    pub fn new_mock(skew_scale: i128) -> Self {
        Self {
            skew_scale: Quantity::new_int(skew_scale),
            max_abs_oi: Quantity::new_int(1_000_000),
            ..Default::default()
        }
    }
}

/// Global state that concerns the counterparty vault and all trading pairs.
#[grug::derive(Serde, Borsh)]
#[derive(Default)]
pub struct State {
    /// Total supply of the vault's share token.
    pub vault_share_supply: Uint128,

    /// The vault margin (LP capital deposited into the exchange). All PnL
    /// settlement, trading fees, liquidation fees, and bad debt flow through
    /// this balance. The vault is a regular trader; its equity is computed
    /// identically to any user via `compute_user_equity`.
    ///
    /// This does not equal the vault's token balance tracked by the bank
    /// contract, which also includes unlocks that are pending cooldown.
    pub vault_margin: Uint128,

    /// Accumulated bad debt that exceeded the vault during liquidations.
    /// When non-zero, ADL can be triggered. Reduced as profitable positions
    /// are forcibly closed and their PnL forfeited.
    pub adl_deficit: Uint128,
}

/// State of an individual trading pair.
#[grug::derive(Serde, Borsh)]
#[derive(Default)]
pub struct PairState {
    /// The sum of the sizes of all long positions.
    pub long_oi: Quantity,

    /// The sum of the absolute value of the sizes of all short positions.
    pub short_oi: Quantity,

    /// The difference between long and short OI. Equals `self.long_oi - self.short_oi`.
    pub skew: Quantity,

    /// Instantaneous funding rate (fraction per day) at the `last_funding_time`.
    ///
    /// Positive = longs pay shorts (and vault collects the net)
    /// Negative = shorts pay longs (and vault collects the net)
    ///
    /// The rate changes linearly over time according to the velocity model:
    ///   rate' = rate + velocity * elapsed_days
    pub funding_rate: FundingRate,

    /// Cumulative funding per unit of position size, denominated in USD.
    ///
    /// This is an ever-increasing accumulator. To compute a position's accrued
    /// funding, take the difference between the current value and the position's
    /// `entry_funding_per_unit`.
    pub funding_per_unit: FundingPerUnit,

    /// Timestamp of the most recent funding accrual.
    pub last_funding_time: Timestamp,
}

impl PairState {
    pub fn new(current_time: Timestamp) -> Self {
        Self {
            last_funding_time: current_time,
            ..Default::default()
        }
    }
}

/// State of a specific user.
#[grug::derive(Serde, Borsh)]
#[derive(Default)]
pub struct UserState {
    /// The user's vault withdrawals that are pending cooldown.
    pub unlocks: VecDeque<Unlock>,

    /// The user's open positions.
    pub positions: BTreeMap<PairId, Position>,

    /// Margin reserved for resting limit orders.
    pub reserved_margin: UsdValue,

    /// Number of resting limit orders the user currently has on the book.
    pub open_order_count: usize,
}

impl UserState {
    /// Return whether the `UserState` is completely empty.
    pub fn is_empty(&self) -> bool {
        self == &Self::default()
    }
}

/// A user's position in a specific trading pair.
#[grug::derive(Serde, Borsh)]
pub struct Position {
    /// The position's size. Position = long, negative = short.
    pub size: Quantity,

    /// The average price at which this position was entered.
    pub entry_price: UsdPrice,

    /// The value of `pair_state.cumulative_funding_per_unit` at the time when
    /// this position was last opened, modified, or funding settled.
    pub entry_funding_per_unit: FundingPerUnit,
}

/// A pending withdrawal of liquidity from the counterparty vault, awaiting the
/// cooldown period to elapse.
#[grug::derive(Serde, Borsh)]
pub struct Unlock {
    /// The amount of settlement currency to be released once cooldown completes.
    pub amount_to_release: Uint128,

    /// The time when cooldown completes.
    pub end_time: Timestamp,
}

/// A resting limit order, waiting to be fulfilled.
///
/// This struct does not contain the pair ID, order ID, and the limit price,
/// which are instead included in the storage key, with which this struct is
/// saved in the contract storage .
#[grug::derive(Serde, Borsh)]
pub struct Order {
    pub user: Addr,
    pub size: Quantity,
    pub reduce_only: bool,
    pub reserved_margin: UsdValue,
}

#[grug::derive(Serde)]
pub enum CancelOrderRequest {
    /// Cancel a single order by ID.
    One(OrderId),
    /// Cancel all orders associated with the sender.
    All,
}

// --------------------------------- Messages ----------------------------------

#[grug::derive(Serde)]
pub struct InstantiateMsg {
    pub param: Param,
    pub pair_params: BTreeMap<PairId, PairParam>,
}

#[grug::derive(Serde)]
pub enum ExecuteMsg {
    /// Add liquidity to the counterparty vault.
    Deposit {
        /// Revert if less than this amount of shares is minted.
        min_shares_to_mint: Option<Uint128>,
    },

    /// Request to withdraw funds from the counterparty vault.
    Unlock {},

    /// Claim funds from unlocks that have already completed cooldown.
    Claim {},

    /// Submit an order.
    SubmitOrder {
        pair_id: PairId,

        /// The amount of futures contract to buy or sell.
        /// Positive indicates buy, negative indicates sell.
        size: Quantity,

        /// Order type: market, limit, etc.
        kind: OrderKind,

        /// If true, the opening portion of the order is discarded, while the
        /// closing portion of the order is always executed, ignoring the risk
        /// parameters such as maximum open interest (OI).
        ///
        /// If false, the order must be executed in full. If any of the risk
        /// parameters is violated, the entire order is aborted.
        reduce_only: bool,
    },

    /// Cancel a resting limit order.
    CancelOrder(CancelOrderRequest),

    /// Forcibly close all of a user's positions, if the user has less collateral
    /// than the maintenance margin required by his positions.
    Liquidate { user: Addr },

    /// Forcibly close all of a user's positions, even if the user has sufficient
    /// amount of collateral.
    ///
    /// This is enabled when the vault cannot fully absorb bad debt from
    /// liquidations.
    Deleverage { user: Addr },
}

#[grug::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    /// Query the global parameters.
    #[returns(Param)]
    Param {},

    /// Query the pair-specific parameters of a single trading pair.
    #[returns(Option<PairParam>)]
    PairParam { pair_id: PairId },

    /// Enumerate the pair-specific parameters of all trading pairs.
    #[returns(BTreeMap<PairId, PairParam>)]
    PairParams {
        start_after: Option<PairId>,
        limit: Option<u32>,
    },

    /// Query the global state.
    #[returns(State)]
    State {},

    /// Query the pair-specific state of a single trading pair.
    #[returns(Option<PairState>)]
    PairState { pair_id: PairId },

    /// Enumerate the pair-specific states of all trading pairs.
    #[returns(BTreeMap<PairId, PairState>)]
    PairStates {
        start_after: Option<PairId>,
        limit: Option<u32>,
    },

    /// Query the state of a single user.
    #[returns(Option<UserState>)]
    UserState { user: Addr },

    /// Enumerate the states of all users.
    #[returns(BTreeMap<Addr, UserState>)]
    UserStates {
        start_after: Option<Addr>,
        limit: Option<u32>,
    },

    /// Query a single order by ID.
    #[returns(Option<QueryOrderResponse>)]
    Order { order_id: OrderId },

    /// Query all orders of a single user.
    #[returns(Vec<QueryOrderResponse>)]
    OrdersByUser { user: Addr },
}

#[grug::derive(Serde)]
pub struct QueryOrderResponse {
    pub order_id: OrderId,
    pub pair_id: PairId,
    pub limit_price: UsdPrice,
    pub size: Quantity,
    pub reduce_only: bool,
    pub reserved_margin: UsdValue,
}

#[grug::derive(Serde)]
pub struct QueryOrdersByUserResponse {
    pub bids: Vec<QueryOrderResponse>,
    pub asks: Vec<QueryOrderResponse>,
}

// ---------------------------------- Events -----------------------------------

// TODO
