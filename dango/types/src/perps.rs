use {
    crate::account_factory::UserIndex,
    dango_order_book::{
        ChildOrder, ClientOrderId, ConditionalOrder, Dimensionless, FillId, FundingPerUnit,
        FundingRate, LiquidityDepthResponse, OrderId, OrderKind, PairId, Quantity,
        QueryOrderResponse, QueryOrdersByUserResponseItem, TriggerDirection, UsdPrice, UsdValue,
    },
    grug::{
        Addr, Duration, MathResult, NonEmpty, Op, Order as IterationOrder, Part, Timestamp, Uint128,
    },
    std::{
        collections::{BTreeMap, BTreeSet, VecDeque},
        sync::LazyLock,
    },
};

/// Perpetual trading pairs are to be registered in the oracle contract. These
/// pairs are namespaced with this value to distinguish from spot assets.
pub static NAMESPACE: LazyLock<Part> = LazyLock::new(|| Part::new_unchecked("perp"));

/// The settlement currency (USDC) is valued at a fixed $1 per unit for deposit
/// and withdrawal conversions.
pub const SETTLEMENT_CURRENCY_PRICE: UsdPrice = UsdPrice::new_int(1);

// ----------------------------------- Types -----------------------------------

/// Denomination of the asset used to settle perpetual futures contracts.
pub use crate::constants::usdc as settlement_currency;

/// Type alias for a referrer's user index.
pub type Referrer = UserIndex;

/// Type alias for a referee's user index.
pub type Referee = UserIndex;

/// The fee share ratio a referrer gives back to the referee.
/// Uses `Dimensionless` (a value in the range appropriate for ratios).
pub type FeeShareRatio = Dimensionless;

/// Commission rate — the fraction of the post-protocol-cut fee distributed
/// to the referral chain when a user with an active referral trades.
pub type CommissionRate = Dimensionless;

/// Referrer settings for a referrer.
#[grug::derive(Serde)]
#[derive(Copy)]
pub struct ReferrerSettings {
    pub commission_rate: CommissionRate,
    pub share_ratio: FeeShareRatio,
}

/// Per-referee statistics tracked from the referrer's perspective.
#[grug::derive(Serde, Borsh)]
#[derive(Default)]
pub struct RefereeStats {
    /// Timestamp when the referee registered with the referral.
    pub registered_at: Timestamp,

    /// Total trading volume made by the referee (USD).
    pub volume: UsdValue,

    /// Total commission earned by the referrer from this referee (USD).
    pub commission_earned: UsdValue,

    /// Timestamp of the last day the referee was active.
    pub last_day_active: Timestamp,
}

/// Ordering options for querying per-referee statistics.
#[grug::derive(Serde)]
pub struct ReferrerStatsOrderBy {
    pub order: IterationOrder,
    pub limit: Option<u32>,
    pub index: ReferrerStatsOrderIndex,
}

/// Which index to use when querying per-referee statistics.
#[grug::derive(Serde)]
pub enum ReferrerStatsOrderIndex {
    Commission { start_after: Option<UsdValue> },
    RegisterAt { start_after: Option<Timestamp> },
    Volume { start_after: Option<UsdValue> },
}

/// A base rate with optional volume-tiered overrides.
///
/// The highest qualifying tier (by volume threshold) wins;
/// if no tier is met, the base rate applies.
#[grug::derive(Serde, Borsh)]
#[derive(Default)]
pub struct RateSchedule {
    /// The rate applied when no volume tier qualifies.
    pub base: Dimensionless,

    /// Volume-tiered rates. Key = minimum USD volume threshold;
    /// value = rate. Highest qualifying tier wins.
    pub tiers: BTreeMap<UsdValue, Dimensionless>,
}

impl RateSchedule {
    /// Resolve the applicable rate for the given volume.
    pub fn resolve(&self, volume: UsdValue) -> Dimensionless {
        self.tiers
            .range(..=volume)
            .next_back()
            .map(|(_, &rate)| rate)
            .unwrap_or(self.base)
    }
}

/// Global parameters that concerns the counterparty vault and all trading pairs.
#[grug::derive(Serde, Borsh)]
#[derive(Default)]
pub struct Param {
    /// Maximum number of unlock requests a single user may have.
    ///
    /// Bounds: `> 0`.
    pub max_unlocks: usize,

    /// Maximum number of resting limit order a single user may have across all
    /// trading pairs.
    ///
    /// Bounds: `> 0`.
    pub max_open_orders: usize,

    /// Volume-tiered maker fee rates. Highest qualifying tier wins;
    /// base rate applies when no tier is met.
    ///
    /// Bounds: base and every tier rate in `[-1, 1]`. Negative values
    /// represent a rebate paid to the maker.
    pub maker_fee_rates: RateSchedule,

    /// Volume-tiered taker fee rates. Highest qualifying tier wins;
    /// base rate applies when no tier is met.
    ///
    /// Bounds: base and all tier rates in `[0, 1]`.
    pub taker_fee_rates: RateSchedule,

    /// Fraction of each trading fee routed to the protocol treasury.
    /// The remainder (1 − `protocol_fee_rate`) stays with the vault.
    ///
    /// Bounds: `[0, 1]`.
    pub protocol_fee_rate: Dimensionless,

    /// Fee paid to the insurance fund as a fraction of the total notional
    /// value of positions being liquidated, capped at the user's remaining
    /// margin after position closure.
    ///
    /// fee = min(
    ///   ceil(|position_size| * oracle_price * liquidation_fee_rate / settlement_currency_price),
    ///   user_remaining_margin
    /// )
    ///
    /// Bounds: `[0, 1]`.
    pub liquidation_fee_rate: Dimensionless,

    /// Buffer applied during liquidation so that the user's post-liquidation
    /// equity slightly exceeds the remaining maintenance margin.
    ///
    /// With a buffer ratio `b`, the deficit formula becomes:
    ///
    ///   deficit = MM - equity / (1 + b)
    ///
    /// When `b = 0`, positions are closed to bring equity exactly to the
    /// maintenance margin boundary (default behavior). Recommended: 5-10%.
    ///
    /// Bounds: `[0, 1)`.
    pub liquidation_buffer_ratio: Dimensionless,

    /// Duration between funding collections. The cron job applies funding
    /// only when this period elapses.
    ///
    /// Bounds: `(0, 7 days]`. The upper bound is a governance guardrail —
    /// a longer period would effectively suspend funding collection.
    pub funding_period: Duration,

    /// Sum of `vault_liquidity_weight` across all trading pairs.
    /// Precomputed to avoid iterating all pair params when placing
    /// vault orders. Must be kept in sync when pairs are added/removed
    /// or weights change.
    ///
    /// Bounds: `>= 0`. Must equal the sum of `vault_liquidity_weight`
    /// across all pairs.
    pub vault_total_weight: Dimensionless,

    /// Once a request to withdraw liquidity from the counterparty vault has been
    /// submitted, the waiting time that must elapsed before the funds are released
    /// to the liquidity provider.
    ///
    /// Bounds: `(0, 30 days]`. The upper bound is a governance guardrail —
    /// a longer cooldown would effectively lock LPs out of their deposits.
    pub vault_cooldown_period: Duration,

    /// Whether the referral commission system is active.
    /// When false, `apply_fee_commissions` is skipped entirely.
    pub referral_active: bool,

    /// Minimum lifetime perps trading volume a user must have
    /// before they can become a referrer by setting a fee share ratio.
    ///
    /// Bounds: `>= 0`. Zero means no minimum.
    pub min_referrer_volume: UsdValue,

    /// Volume-tiered referrer commission rates. Highest qualifying tier wins;
    /// base rate applies when no tier is met.
    ///
    /// Bounds: base and all tier rates in `[0, 1]`.
    pub referrer_commission_rates: RateSchedule,

    /// Maximum total margin the counterparty vault may hold. Deposits that
    /// would push `vault_margin + deposit` above this cap are rejected.
    /// `None` means no cap (unlimited deposits).
    ///
    /// Bounds: if `Some`, `> 0`. Use `None` for unlimited.
    pub vault_deposit_cap: Option<UsdValue>,

    /// Maximum number of actions allowed in a single
    /// `TraderMsg::BatchUpdateOrders` message.
    ///
    /// Bounds: `>= 1`. Governance-tunable via `Configure`; no hard
    /// upper bound is enforced.
    pub max_action_batch_size: usize,
}

/// Global state that concerns the counterparty vault and all trading pairs.
#[grug::derive(Serde, Borsh)]
#[derive(Default)]
pub struct State {
    /// Timestamp of the most recent funding collection.
    pub last_funding_time: Timestamp,

    /// Total supply of the vault's share token.
    pub vault_share_supply: Uint128,

    /// Insurance fund balance, funded by liquidation fees and used to cover
    /// bad debt. May go negative when bad debt exceeds the fund; future
    /// liquidation fees will replenish it.
    pub insurance_fund: UsdValue,

    /// Accumulated protocol fees from trading. Incremented on every fill
    /// settlement by `fee * protocol_fee_rate`.
    pub treasury: UsdValue,
}

/// Parameters that apply to an individual trading pair.
#[grug::derive(Serde, Borsh)]
#[derive(Default)]
pub struct PairParam {
    /// Minimum price increment for limit orders in this pair. All limit order
    /// prices must be an integer multiple of `tick_size`.
    ///
    /// Bounds: `> 0`. Used as a divisor in price snapping.
    pub tick_size: UsdPrice,

    /// Minimum notional value for an order. Reduce-only orders are exempt.
    /// Prevents dust orders from cluttering the order book.
    ///
    /// Bounds: `>= 0`. Zero disables the minimum.
    pub min_order_size: UsdValue,

    /// Maximum deviation of a limit order's `limit_price` from the oracle
    /// price, expressed as a fraction. A limit order is accepted only if
    ///
    /// ```plain
    /// |limit_price - oracle_price| / oracle_price <= max_limit_price_deviation
    /// ```
    ///
    /// This prevents users from placing resting orders at pathological prices
    /// (e.g. 99% below oracle) that could trap counterparties into bad-price
    /// fills.
    ///
    /// Bounds: `(0, 1)`.
    ///
    /// Cross-field invariant:
    /// `max_limit_price_deviation >= vault_half_spread * (1 + vault_spread_skew_factor)`.
    /// The band must be at least as wide as the vault's widest quote
    /// deviation under maximum skew, otherwise users' crossing limit
    /// orders at the vault's legitimately-quoted edges would be rejected.
    /// Enforced at `Configure` time.
    pub max_limit_price_deviation: Dimensionless,

    /// Maximum slippage tolerance a user may specify on a market order or
    /// a TP/SL child order in this pair. Market orders compute their
    /// `target_price` as `oracle_price * (1 ± max_slippage)`; this field
    /// caps how far the user may push that target at submission.
    ///
    /// Acts as the submission-time analogue of `max_limit_price_deviation`
    /// for limit orders.
    ///
    /// Bounds: `(0, 1)`. Aligns with industry practice (dYdX 10% flat,
    /// Hyperliquid 10% for TP/SL).
    pub max_market_slippage: Dimensionless,

    /// The maximum allowed open interest for both long and short.
    /// I.e. the following must be satisfied:
    ///
    /// pair_state.long_oi <= max_abs_oi && pair_state.short_oi <= max_abs_oi
    ///
    /// This constraint does not apply to reduce-only orders.
    ///
    /// Bounds: `> 0`.
    pub max_abs_oi: Quantity,

    /// Maximum absolute funding rate, as a fraction per day.
    ///
    /// That is, the daily funding rate is clamped to the range
    /// [-max_abs_funding_rate, max_abs_funding_rate].
    ///
    /// This prevents runaway rates from causing cascading liquidations and bad
    /// debt spirals during prolonged skew.
    ///
    /// Bounds: `[0, 1]`. 100% per day is already pathological — the upper
    /// bound is a governance guardrail.
    pub max_abs_funding_rate: FundingRate,

    /// Margin requirement when opening or increasing a position in this trading
    /// pair. E.g. 5% indicates a 1 / 5% = 20x maximum leverage.
    ///
    /// initial_margin = |position_size| * oracle_price * initial_margin_ratio
    ///
    /// Bounds: `(0, 1]`. Must be `> maintenance_margin_ratio`.
    /// Used as a divisor in vault quote size computation.
    pub initial_margin_ratio: Dimensionless,

    /// Margin requirement for maintaining a position in this trading pair.
    ///
    /// When a user's equity falls below the sum of maintenance margins across
    /// all his positions, the user becomes eligible for liquidations.
    ///
    /// maintenance_margin = |position_size| * oracle_price * maintenance_margin_ratio
    ///
    /// Bounds: `(0, 1]`. Must be `< initial_margin_ratio`.
    pub maintenance_margin_ratio: Dimensionless,

    /// Notional value used to compute impact prices from the order book.
    /// The cron job walks bids/asks to find the average execution price for
    /// selling/buying this much notional.
    ///
    /// Bounds: `> 0`.
    pub impact_size: UsdValue,

    /// Weight determining what fraction of the vault's available margin
    /// is allocated to this pair for market-making.
    /// The pair's share = `vault_liquidity_weight / Param::vault_total_weight`.
    ///
    /// Bounds: `>= 0`. Zero disables vault quoting for this pair.
    pub vault_liquidity_weight: Dimensionless,

    /// Half the bid-ask spread the vault quotes around the oracle price. The
    /// vault places bids at `oracle_price * (1 - vault_half_spread)` and asks
    /// at `oracle_price * (1 + vault_half_spread)`.
    ///
    /// Bounds: `(0, 1)`. Zero would collapse bid and ask onto the oracle
    /// price; `>= 1` would produce a non-positive bid price. To disable the
    /// vault for a pair, set `vault_liquidity_weight = 0` instead.
    ///
    /// Cross-field invariant: `vault_half_spread * (1 + vault_spread_skew_factor) < 1`.
    /// Under maximum positive inventory skew the bid's effective spread
    /// widens by this factor, and the product must stay strictly below 1
    /// so the bid price never collapses to or below zero.
    pub vault_half_spread: Dimensionless,

    /// Maximum notional size (in quote currency) of the vault's resting orders
    /// on each side of the book. Limits the vault's exposure per pair.
    ///
    /// Bounds: `> 0`. To disable the vault for a pair, set
    /// `vault_liquidity_weight = 0` instead.
    pub vault_max_quote_size: Quantity,

    /// How aggressively to tilt order sizes based on inventory.
    /// 0 = no skew (symmetric), 1 = fully stop quoting on one side at max
    /// position.
    ///
    /// Bounds: `[0, 1]`. Values > 1 would produce negative order sizes.
    pub vault_size_skew_factor: Dimensionless,

    /// How aggressively to tilt spreads based on inventory.
    /// 0 = no skew (symmetric spreads on both sides).
    ///
    /// Bounds: `>= 0`. Values > 1 cause the tightened side's quote to
    /// cross the oracle price at maximum skew — e.g. when the vault is
    /// fully long, the ask drops below the oracle. This is an aggressive
    /// unwind posture, useful when the vault needs to rapidly deleverage
    /// a large directional position. The invariant `bid < ask` always
    /// holds because `ask - bid = 2 * oracle_price * vault_half_spread`,
    /// independent of this factor.
    ///
    /// The effective upper bound is enforced via the cross-field invariant
    /// `vault_half_spread * (1 + vault_spread_skew_factor) < 1` documented
    /// on `vault_half_spread`, which prevents the bid price from collapsing
    /// to zero at maximum positive skew.
    pub vault_spread_skew_factor: Dimensionless,

    /// Position size at which inventory skew saturates.
    /// When the vault's position reaches this size, skew is at maximum (±1).
    ///
    /// Bounds: `>= 0`. Zero disables inventory skew (skew always 0).
    pub vault_max_skew_size: Quantity,

    /// Multiplier applied to the funding-rate premium so governance can tune
    /// funding independently of the vault's quoting parameters. The full
    /// formula is:
    ///
    /// ```plain
    /// premium = -halfSpread × skew × spreadSkewFactor × fundingRateMultiplier
    /// ```
    ///
    /// `1` reproduces the pre-multiplier behavior; `0` disables funding
    /// without touching the vault's quoting; values `> 1` amplify funding.
    ///
    /// Bounds: `>= 0`. Negative values would flip the funding sign and
    /// invert the economic incentive.
    pub funding_rate_multiplier: Dimensionless,

    /// Price bucket sizes for which aggregated order book depth is maintained.
    /// Each entry defines a granularity level for the depth query.
    ///
    /// Bounds: each entry `> 0`.
    pub bucket_sizes: BTreeSet<UsdPrice>,
}

impl PairParam {
    /// Build a `PairParam` with sensible defaults for testing.
    pub fn new_mock() -> Self {
        Self {
            max_abs_oi: Quantity::new_int(1_000_000),
            impact_size: UsdValue::new_int(10_000),
            vault_half_spread: Dimensionless::new_permille(10), // 1%
            vault_max_quote_size: Quantity::new_int(100),
            max_limit_price_deviation: Dimensionless::new_permille(500), // 50%
            max_market_slippage: Dimensionless::new_permille(500),       // 50%
            funding_rate_multiplier: Dimensionless::ONE,
            ..Default::default()
        }
    }
}

/// State of an individual trading pair.
#[grug::derive(Serde, Borsh)]
#[derive(Default)]
pub struct PairState {
    /// The sum of the sizes of all long positions.
    pub long_oi: Quantity,

    /// The sum of the absolute value of the sizes of all short positions.
    pub short_oi: Quantity,

    /// Cumulative funding per unit of position size, denominated in USD.
    ///
    /// This is an ever-increasing accumulator. To compute a position's accrued
    /// funding, take the difference between the current value and the position's
    /// `entry_funding_per_unit`.
    pub funding_per_unit: FundingPerUnit,

    /// The clamped per-day funding rate applied during the most recent funding
    /// collection. Positive means longs pay shorts; negative means shorts pay
    /// longs.
    pub funding_rate: FundingRate,
}

/// State of a specific user. Saved in contract storage.
#[grug::derive(Serde, Borsh)]
#[derive(Default)]
pub struct UserState {
    /// The user's deposited margin, denominated in USD.
    pub margin: UsdValue,

    /// Vault shares owned by this user.
    pub vault_shares: Uint128,

    /// The user's open positions.
    pub positions: BTreeMap<PairId, Position>,

    /// The user's vault withdrawals that are pending cooldown.
    pub unlocks: VecDeque<Unlock>,

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

/// State of a specific user, containing a few fields not saved in contract
/// storage but rather computed on-the-fly at query time. Used in query response.
#[grug::derive(Serde)]
pub struct UserStateExtended {
    // These are the same fields from the raw `UserState`, except for `positions`
    // which is replaced with `PositionExtended` (see below).
    pub margin: UsdValue,
    pub vault_shares: Uint128,
    pub unlocks: VecDeque<Unlock>,
    pub reserved_margin: UsdValue,
    pub open_order_count: usize,

    /// The user's equity, defined as:
    ///
    /// ```plain
    /// margin + sum_all_pairs(unrealized_pnl - unrealized_funding)
    /// ```
    ///
    /// Equity reflects the total value of the account's collaterals and positions.
    ///
    /// `None` if the client elects not to compute this in `QueryMsg::UserStateExtended`.
    pub equity: Option<UsdValue>,

    /// The user's available margin, defined as:
    ///
    /// ```plain
    /// margin
    ///   - sum_all_pairs(|size| * oracle_price * initial_margin_ratio)
    ///   - sum_all_orders(reserved_margin)
    /// ```
    ///
    /// I.e. the user's collateral (margin) minus the cost for opening his all
    /// existing positions and resting orders.
    ///
    /// This is the amount of collateral that the user can 1) withdraw to spot
    /// account, or 2) use to place orders. E.g. if the user has $100 available
    /// margin, it means he can either withdraw up to 100 USDC to his spot account,
    /// or if he has set leverage to 5x, he can open a new position of size up to
    /// $100 * 5 = $500.
    ///
    /// `None` if the client elects not to compute this in `QueryMsg::UserStateExtended`.
    pub available_margin: Option<UsdValue>,

    /// The user's maintenance margin.
    ///
    /// Defined as:
    ///
    /// ```plain
    /// sum_all_pairs(|position.size| * oracle_price * maintenance_margin_ratio)
    /// ```
    ///
    /// Used as liquidation trigger: if equity < maintenance margin, the user is
    /// liquidatable.
    pub maintenance_margin: Option<UsdValue>,

    /// The user's open positions, enriched with optional computed data.
    pub positions: BTreeMap<PairId, PositionExtended>,
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

    /// Conditional order that triggers when oracle_price >= trigger_price.
    /// Used for: TP on longs, SL on shorts.
    pub conditional_order_above: Option<ConditionalOrder>,

    /// Conditional order that triggers when oracle_price <= trigger_price.
    /// Used for: SL on longs, TP on shorts.
    pub conditional_order_below: Option<ConditionalOrder>,
}

/// A user's position enriched with optional computed data.
///
/// Contains all fields from `Position` plus optional position-level metrics
/// computed on-the-fly at query time.
#[grug::derive(Serde)]
pub struct PositionExtended {
    // These are the same data from the raw `Position`.
    pub size: Quantity,
    pub entry_price: UsdPrice,
    pub entry_funding_per_unit: FundingPerUnit,
    pub conditional_order_above: Option<ConditionalOrder>,
    pub conditional_order_below: Option<ConditionalOrder>,

    /// Unrealized PnL for this position:
    ///
    /// ```plain
    /// size * (oracle_price - entry_price)
    /// ```
    ///
    /// Positive = profit, negative = loss. Sign accounts for direction.
    ///
    /// `None` if the client elects not to compute this in `QueryMsg::UserStateExtended`.
    pub unrealized_pnl: Option<UsdValue>,

    /// Unrealized funding accrued by this position since it was last touched:
    ///
    /// ```plain
    /// size * (current_funding_per_unit - entry_funding_per_unit)
    /// ```
    ///
    /// Positive = trader owes (cost), negative = trader is owed (credit).
    ///
    /// `None` if the client elects not to compute this in `QueryMsg::UserStateExtended`.
    pub unrealized_funding: Option<UsdValue>,

    /// The oracle price of this pair at which the account-level liquidation
    /// condition triggers, assuming all other pair prices remain constant
    /// (cross-margin partial-derivative approach).
    ///
    /// `None` if the client elects not to compute this, or if no valid
    /// liquidation price exists (the position alone cannot trigger liquidation).
    pub liquidation_price: Option<UsdPrice>,
}

/// A pending withdrawal of liquidity from the counterparty vault, awaiting the
/// cooldown period to elapse.
#[grug::derive(Serde, Borsh)]
pub struct Unlock {
    /// The time when cooldown completes.
    pub end_time: Timestamp,

    /// The USD value to be released once cooldown completes. Token conversion
    /// happens at claim time using the current oracle price.
    pub amount_to_release: UsdValue,
}

/// Cumulative referral data for a user, bucketed by day.
///
/// Each day-bucket stores running totals that can be differenced to compute
/// rolling-window values (e.g. 30-day referees volume).
#[grug::derive(Serde, Borsh)]
#[derive(Default)]
pub struct UserReferralData {
    /// The user's own trading volume (cumulative).
    pub volume: UsdValue,

    /// Total commission shared by this user's referrer (cumulative).
    pub commission_shared_by_referrer: UsdValue,

    /// Number of direct referees this user has.
    pub referee_count: u32,

    /// Total trading volume of this user's direct referees (cumulative).
    pub referees_volume: UsdValue,

    /// Total commission distributed from this user's referees (cumulative).
    pub commission_earned_from_referees: UsdValue,

    /// Cumulative count of daily active direct referees. Incremented by one
    /// each time a direct referee trades for the first time on a given day.
    /// Difference two buckets to get the count for a specific window.
    pub cumulative_daily_active_referees: u32,

    /// Number of direct referees that have made at least one trade.
    /// Incremented once per referee, on their very first trade.
    pub cumulative_global_active_referees: u32,
}

impl UserReferralData {
    /// Element-wise subtraction for rolling window calculations.
    pub fn checked_sub(&self, other: &Self) -> MathResult<Self> {
        Ok(Self {
            volume: self.volume.checked_sub(other.volume)?,
            commission_shared_by_referrer: self
                .commission_shared_by_referrer
                .checked_sub(other.commission_shared_by_referrer)?,
            referee_count: self.referee_count.saturating_sub(other.referee_count),
            referees_volume: self.referees_volume.checked_sub(other.referees_volume)?,
            commission_earned_from_referees: self
                .commission_earned_from_referees
                .checked_sub(other.commission_earned_from_referees)?,
            cumulative_daily_active_referees: self
                .cumulative_daily_active_referees
                .saturating_sub(other.cumulative_daily_active_referees),
            cumulative_global_active_referees: self
                .cumulative_global_active_referees
                .saturating_sub(other.cumulative_global_active_referees),
        })
    }
}

/// Parameters for submitting an order. Shared between
/// `TraderMsg::SubmitOrder` and (upcoming) `TraderMsg::BatchUpdateOrders`
/// so the two message variants carry exactly the same shape.
#[grug::derive(Serde)]
pub struct SubmitOrderRequest {
    pub pair_id: PairId,

    /// The amount of futures contract to buy or sell.
    /// Positive indicates buy, negative indicates sell.
    pub size: Quantity,

    /// Order type: market, limit, etc.
    pub kind: OrderKind,

    /// If true, the opening portion of the order is discarded, while the
    /// closing portion of the order is always executed, ignoring the risk
    /// parameters such as maximum open interest (OI).
    ///
    /// If false, the order must be executed in full. If any of the risk
    /// parameters is violated, the entire order is aborted.
    pub reduce_only: bool,

    /// Take-profit child order. Applied to the resulting position after fill.
    pub tp: Option<ChildOrder>,

    /// Stop-loss child order. Applied to the resulting position after fill.
    pub sl: Option<ChildOrder>,
}

#[grug::derive(Serde)]
pub enum CancelOrderRequest {
    /// Cancel a single order by its system-assigned `OrderId`.
    One(OrderId),

    /// Cancel a single order by its caller-assigned `ClientOrderId`.
    /// Resolves to the active order owned by the sender that carries this
    /// client id; bails if no such order exists.
    OneByClientOrderId(ClientOrderId),

    /// Cancel all orders associated with the sender.
    All,
}

/// One action inside a `TraderMsg::BatchUpdateOrders` list.
///
/// Conditional (TP/SL) orders are intentionally out of scope for
/// batching — use `SubmitConditionalOrder` / `CancelConditionalOrder`.
#[grug::derive(Serde)]
#[allow(clippy::large_enum_variant)]
pub enum SubmitOrCancelOrderRequest {
    Submit(SubmitOrderRequest),
    Cancel(CancelOrderRequest),
}

#[grug::derive(Serde)]
pub enum CancelConditionalOrderRequest {
    /// Cancel a single conditional order identified by pair and direction.
    One {
        pair_id: PairId,
        trigger_direction: TriggerDirection,
    },

    /// Cancel all conditional orders for a specific pair.
    AllForPair { pair_id: PairId },

    /// Cancel all conditional orders associated with the sender.
    All,
}

/// A point-in-time snapshot of the market-making vault's redemption value.
///
/// Stored daily by the cron handler. The ratio `equity / share_supply` is the
/// redemption value of one vault share, and how it varies over time describes
/// the vault's historical profitability.
#[grug::derive(Serde, Borsh)]
pub struct VaultSnapshot {
    pub equity: UsdValue,
    pub share_supply: Uint128,
}

// --------------------------------- Messages ----------------------------------

#[grug::derive(Serde)]
pub struct InstantiateMsg {
    pub param: Param,
    pub pair_params: BTreeMap<PairId, PairParam>,
}

#[grug::derive(Serde)]
#[allow(clippy::large_enum_variant)]
pub enum ExecuteMsg {
    /// Messages for contract maintenance (owner/admin).
    Maintain(MaintainerMsg),

    /// Messages related to trading.
    Trade(TraderMsg),

    /// Messages related to the market making vault.
    Vault(VaultMsg),

    /// Messages related to the referral system.
    Referral(ReferralMsg),
}

#[grug::derive(Serde)]
#[allow(clippy::large_enum_variant)]
pub enum MaintainerMsg {
    /// Update global and/or per-pair parameters.
    /// Only callable by the chain owner (or GENESIS_SENDER during instantiation).
    Configure {
        param: Param,
        pair_params: BTreeMap<PairId, PairParam>,
    },

    /// Forcibly close all of a user's positions, if the user has less collateral
    /// than the maintenance margin required by his positions.
    ///
    /// Unfilled positions are ADL'd against counter-parties at the bankruptcy
    /// price. Any remaining bad debt is absorbed by the insurance fund.
    Liquidate { user: Addr },

    /// Accept a USDC donation to the perps contract.
    /// Only callable by the chain owner. Must attach exactly USDC, nonzero.
    Donate {},

    /// Override a user's fee rate, overriding the tier-based fee rates derived
    /// from the user's recent trading volume.
    ///
    /// Bounds:
    ///
    /// - `maker_fee_rate`: [-1, 1]. Note that negative maker rates are allowed.
    /// - `taker_fee_rate`: [0, 1].
    SetFeeRateOverride {
        user: Addr,

        /// First element is maker rate, second is taker rate.
        maker_taker_fee_rates: Op<(Dimensionless, Dimensionless)>,
    },

    /// Move the entire `state.treasury` balance into the chain owner's
    /// `UserState.margin`. The owner can then convert it to USDC via the
    /// regular `TraderMsg::Withdraw` flow.
    /// Only callable by the chain owner.
    WithdrawFromTreasury {},
}

#[grug::derive(Serde)]
pub enum TraderMsg {
    /// Deposit settlement currency into the trader's margin account.
    /// The deposited tokens are converted to USD at the current oracle price
    /// and credited to `user_state.margin`.
    Deposit {
        /// Optional: the perp account address this deposit should go to.
        /// Default to the sender himself's.
        to: Option<Addr>,
    },

    /// Withdraw margin from the trader's margin account.
    /// The requested USD amount is converted to settlement currency at the
    /// current oracle price (floor-rounded) and transferred to the user.
    Withdraw { amount: UsdValue },

    /// Submit an order.
    SubmitOrder(SubmitOrderRequest),

    /// Cancel a resting limit order.
    CancelOrder(CancelOrderRequest),

    /// Execute a sequence of order actions atomically.
    ///
    /// Actions are applied sequentially — later actions observe the state
    /// written by earlier ones — and the whole batch is atomic: if any
    /// action fails the message reverts and no partial state is persisted.
    ///
    /// The list must be non-empty, and its length must not exceed
    /// `Param::max_action_batch_size`.
    ///
    /// Conditional (TP/SL) orders are not supported in batches — use
    /// `SubmitConditionalOrder` / `CancelConditionalOrder`.
    BatchUpdateOrders(NonEmpty<Vec<SubmitOrCancelOrderRequest>>),

    /// Submit a conditional (TP/SL) order that triggers when the oracle price
    /// crosses the specified trigger price. Always reduce-only, executed as a
    /// market order at trigger time.
    SubmitConditionalOrder {
        pair_id: PairId,
        /// If `None`, closes the entire position at trigger time.
        size: Option<Quantity>,
        trigger_price: UsdPrice,
        trigger_direction: TriggerDirection,
        max_slippage: Dimensionless,
    },

    /// Cancel one or more conditional orders.
    CancelConditionalOrder(CancelConditionalOrderRequest),
}

#[grug::derive(Serde)]
pub enum VaultMsg {
    /// Add liquidity to the counterparty vault by transferring margin to the vault.
    AddLiquidity {
        /// USD margin amount to transfer from the user's trading margin to the vault.
        amount: UsdValue,

        /// Revert if less than this amount of shares is minted.
        min_shares_to_mint: Option<Uint128>,
    },

    /// Request to withdraw liquidity from the counterparty vault.
    RemoveLiquidity { shares_to_burn: Uint128 },

    /// Refresh vault market-making orders. Triggered at the beginning of each
    /// block, right after the oracle update.
    ///
    /// The vault places new orders based on the oracle price, the state of the
    /// order book at the time, and its policy for market making.
    Refresh {},
}

#[grug::derive(Serde)]
#[allow(clippy::enum_variant_names)]
pub enum ReferralMsg {
    /// Register a referral relationship between a referrer and a referee.
    ///
    /// Called by the account factory during user registration, or by the
    /// referee himself.
    SetReferral {
        referrer: UserIndex,
        referee: UserIndex,
    },

    /// Set or update the fee share ratio for the calling referrer.
    ///
    /// The share ratio determines what fraction of the commission the referrer
    /// gives back to the referee.
    ///
    /// Can only increase (never decrease) once set.
    SetFeeShareRatio { share_ratio: FeeShareRatio },

    /// Set or remove a commission rate override for a user.
    ///
    /// Only callable by the chain owner.
    ///
    /// Use `Op::Insert` to set, `Op::Delete` to remove.
    SetCommissionRateOverride {
        user: UserIndex,
        commission_rate: Op<CommissionRate>,
    },

    /// Forcibly set a user's fee share ratio.
    ///
    /// Only callable by the chain owner.
    ///
    /// A referrer is only allowed to increase his fee share ratio, not decreasing
    /// it. This is to prevent malicious referrers from rugpull their referees.
    /// However, if a referrer makes an honest mistake setting the wrong ratio,
    /// the owner can step in to correct it.
    ForceSetFeeShareRatio {
        user: UserIndex,
        share_ratio: FeeShareRatio,
    },
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

    /// Query the state of a single user with additional data computed on-the-fly.
    #[returns(UserStateExtended)]
    UserStateExtended {
        user: Addr,

        #[serde(default)]
        include_equity: bool,

        #[serde(default)]
        include_available_margin: bool,

        #[serde(default)]
        include_maintenance_margin: bool,

        #[serde(default)]
        include_unrealized_pnl: bool,

        #[serde(default)]
        include_unrealized_funding: bool,

        #[serde(default)]
        include_liquidation_price: bool,

        /// If `include_all` is true, all fields are computed, overriding other
        /// options (even they are false).
        #[serde(default)]
        include_all: bool,
    },

    /// Enumeate the states of all users with additional data computed on-the-fly.
    #[returns(BTreeMap<Addr, UserStateExtended>)]
    UserStatesExtended {
        start_after: Option<Addr>,
        limit: Option<u32>,

        #[serde(default)]
        include_equity: bool,

        #[serde(default)]
        include_available_margin: bool,

        #[serde(default)]
        include_maintenance_margin: bool,

        #[serde(default)]
        include_unrealized_pnl: bool,

        #[serde(default)]
        include_unrealized_funding: bool,

        #[serde(default)]
        include_liquidation_price: bool,

        #[serde(default)]
        include_all: bool,
    },

    /// Query a single limit order by ID.
    #[returns(Option<QueryOrderResponse>)]
    Order { order_id: OrderId },

    /// Query all limit orders of a single user.
    #[returns(BTreeMap<OrderId, QueryOrdersByUserResponseItem>)]
    OrdersByUser { user: Addr },

    /// Query aggregated order book depth at a specific bucket size.
    #[returns(LiquidityDepthResponse)]
    LiquidityDepth {
        pair_id: PairId,
        bucket_size: UsdPrice,
        limit: Option<u32>,
    },

    /// Query a user's cumulative trading volume by address.
    /// `since: None` -> lifetime volume. `since: Some(ts)` -> volume since ts.
    #[returns(UsdValue)]
    Volume {
        user: Addr,
        since: Option<Timestamp>,
    },

    /// Query a user's cumulative trading volume by user index.
    /// Resolves the `UserIndex` to the master account address via the account
    /// factory, then returns cumulative volume.
    /// `since: None` -> lifetime volume. `since: Some(ts)` -> volume since ts.
    #[returns(UsdValue)]
    VolumeByUser {
        user: UserIndex,
        since: Option<Timestamp>,
    },

    /// Enumerate daily snapshots of the market-making vault's
    /// `(equity, share_supply)`. `min` and `max` are both inclusive timestamp
    /// bounds.
    #[returns(BTreeMap<Timestamp, VaultSnapshot>)]
    VaultSnapshots {
        min: Option<Timestamp>,
        max: Option<Timestamp>,
    },

    /// Query the referrer of a given referee.
    #[returns(Option<Referrer>)]
    Referrer { referee: UserIndex },

    /// Query referral data for a user.
    /// `since: None` -> lifetime data. `since: Some(ts)` -> data since ts.
    #[returns(UserReferralData)]
    ReferralData {
        user: UserIndex,
        since: Option<Timestamp>,
    },

    /// Query multiple cumulative referral data snapshots for a user,
    /// paginated by day-bucket in reverse chronological order. Returns raw
    /// cumulative entries (no delta computation), useful for charting
    /// historical referral activity.
    /// `start_after` — exclusive lower bound for descending iteration;
    /// entries strictly older than this timestamp are returned.
    /// `limit` — max entries to return (defaults to `DEFAULT_PAGE_LIMIT`).
    #[returns(Vec<(Timestamp, UserReferralData)>)]
    ReferralDataEntries {
        user: UserIndex,
        start_after: Option<Timestamp>,
        limit: Option<u32>,
    },

    /// Query per-referee statistics for a referrer, with ordering and pagination.
    #[returns(Vec<(Referee, RefereeStats)>)]
    ReferrerToRefereeStats {
        referrer: Referrer,
        order_by: ReferrerStatsOrderBy,
    },

    /// Return the referral settings if the user is a referrer. Otherwise, return `None`.
    #[returns(Option<ReferrerSettings>)]
    ReferralSettings { user: UserIndex },

    /// Return the commission rate override for a user, if one exists.
    #[returns(Option<CommissionRate>)]
    CommissionRateOverride { user: UserIndex },

    /// Enumerate all commission rate overrides, with pagination.
    #[returns(BTreeMap<UserIndex, CommissionRate>)]
    CommissionRateOverrides {
        start_after: Option<UserIndex>,
        limit: Option<u32>,
    },

    /// Return the trading fee rate override for a user, if one exists.
    /// Return value is a tuple: `[maker_fee_rate, taker_fee_rate]`.
    #[returns(Option<(Dimensionless, Dimensionless)>)]
    FeeRateOverride { user: Addr },

    /// Enumerate all trading fee overrides, with pagination.
    /// Each value in the returned map is a tuple: `[maker_fee_rate, taker_fee_rate]`.
    #[returns(BTreeMap<Addr, (Dimensionless, Dimensionless)>)]
    FeeRateOverrides {
        start_after: Option<Addr>,
        limit: Option<u32>,
    },
}

// ---------------------------------- Events -----------------------------------

// Events are emitted when:
//
// 1. **Push notifications**: Users need to be notified (e.g. fills, liquidation).
// 2. **Indexing**: Data needs to be queryable.
//    - For **users**: order history (`OrderPersisted`, `OrderRemoved(Canceled)`,
//      `OrderFilled`) and PnL history (`OrderFilled`, `Liquidated`, `Deleveraged`).
//    - For the **vault**: PnL history only (`OrderFilled`; vault can't be
//      liquidated or ADL'd).
//
// Events are suppressed when not needed for either purpose:
//
// - Vault order lifecycle (`OrderPersisted`, `OrderRemoved(Canceled)`,
//   `OrderRemoved(Filled)`) is internal churn (every block) — noise for indexers.
// - Liquidation taker uses `OrderId::ZERO` as sentinel (no user-submitted order).
// - ADL closures are off-book — not `OrderFilled` events.
//
// | Event                    | User orders | Vault orders (maker) | Liq. taker            |
// |--------------------------|-------------|----------------------|-----------------------|
// | `Deposited`              | -           | -                    | -                     |
// | `Withdrew`               | -           | -                    | -                     |
// | `LiquidityAdded`         | -           | -                    | -                     |
// | `LiquidityUnlocking`     | -           | -                    | -                     |
// | `LiquidityReleased`      | -           | -                    | -                     |
// | `OrderFilled`            | Yes         | Yes                  | Book-matched only (*) |
// | `OrderPersisted`         | Yes         | No  (placed directly)| -                     |
// | `OrderRemoved(Canceled)` | Yes         | No  (suppressed)     | -                     |
// | `OrderRemoved(Filled)`   | Yes         | No  (suppressed)     | -                     |
// | `OrderRemoved(STP)`      | Yes         | -                    | -                     |
// | `OrderRemoved(Liq.)`     | Yes         | -                    | -                     |
// | `OrderRemoved(ADL)`      | Yes         | -                    | -                     |
// | `Liquidated`             | 1 per pair  | -                    | 1 per pair            |
// | `Deleveraged`            | 1 per ADL'd counter-party          | -                     |
// | `BadDebtCovered`         | 1 per liquidation (if bad debt)    | -                     |
//
// (*) Off-book fills that realize PnL without emitting `OrderFilled`:
//
// - **ADL** — both the liquidated user and counter-parties. `settle_fill` is
//   called with `None` for events on both sides. The liquidated user's ADL
//   is reported via `Liquidated::adl_size/adl_price`. Counter-parties are
//   reported via `Deleveraged` events.
//
// For liquidation, the market-order PnL is captured by `OrderFilled` events,
// the ADL portion by `Liquidated`, and counter-party impact by `Deleveraged`.

/// Event indicating a user has deposited margin into his perp account.
#[grug::event("deposited")]
#[grug::derive(Serde)]
pub struct Deposited {
    /// This means the user whose perp account received the deposit, who is
    /// usually, but not necessarily always, the user who makes the deposit.
    pub user: Addr,
    pub amount: UsdValue,
}

/// Event indicating a user has withdrawn margin from his perp account.
#[grug::event("withdrew")]
#[grug::derive(Serde)]
pub struct Withdrew {
    pub user: Addr,
    pub amount: UsdValue,
}

/// Event indicating a user has deposited liquidity from his perp account margin
/// into the vault.
#[grug::event("liquidity_added")]
#[grug::derive(Serde)]
pub struct LiquidityAdded {
    pub user: Addr,
    pub amount: UsdValue,
    pub shares_minted: Uint128,
}

/// Event indicating a user has initiated unlocking of liquidity from the vault.
#[grug::event("liquidity_unlocking")]
#[grug::derive(Serde)]
pub struct LiquidityUnlocking {
    pub user: Addr,
    pub amount: UsdValue,
    pub shares_burned: Uint128,
    pub end_time: Timestamp,
}

/// Event indicating a user's vault unlock has matured and the released USD
/// value has been credited back to their trading margin.
#[grug::event("liquidity_released")]
#[grug::derive(Serde)]
pub struct LiquidityReleased {
    pub user: Addr,
    pub amount: UsdValue,
}

/// Event indicating an order has been partially or fully filled.
///
/// `closing_size` and `opening_size` correspond to the output of `decompose_fill`.
/// They should have the same sign as, and sum up to, `fill_size`.
/// For maker orders, their signs correspond to that of the maker order itself,
/// not that of the taker order.
///
/// Examples (user is long 10):
///
/// - sell 4  → fill_size = -4,  closing_size = -4,  opening_size =  0
/// - sell 10 → fill_size = -10, closing_size = -10, opening_size =  0
/// - sell 15 → fill_size = -15, closing_size = -10, opening_size = -5 (flip to short 5)
/// - buy  5  → fill_size =  5,  closing_size =  0,  opening_size =  5 (increase long)
#[grug::event("order_filled")]
#[grug::derive(Serde)]
pub struct OrderFilled {
    pub order_id: OrderId,
    pub pair_id: PairId,
    pub user: Addr,
    pub fill_price: UsdPrice,
    pub fill_size: Quantity,
    pub closing_size: Quantity,
    pub opening_size: Quantity,

    /// PnL realized by the user on this fill from price movement on the
    /// closed portion: `|closing_size| * (fill_price - entry_price)` for
    /// longs, mirrored for shorts. Zero on pure-opening fills.
    ///
    /// Prior to v0.17.0 (exclusive) this field also bundled the funding
    /// settled on the user's pre-existing position immediately before the
    /// fill. From v0.17.0 (inclusive) onward, that funding component is
    /// reported separately as `realized_funding` and is no longer included
    /// here.
    ///
    /// Does not include trading fees (see `fee`).
    pub realized_pnl: UsdValue,

    /// Funding settled on the user's pre-existing position immediately
    /// before this fill is applied. Can be non-zero even on pure-opening
    /// fills if the user already held a position. Zero if the user had no
    /// prior position in this pair.
    ///
    /// `None` for trades executed before v0.17.0 — funding was reported as
    /// part of `realized_pnl` prior to that release.
    pub realized_funding: Option<UsdValue>,

    /// Trading fee charged on this fill, in USD. Always non-negative.
    /// Already deducted from the user's margin in the same transaction;
    /// reported here for transparency. Vault fills are exempt and report
    /// zero.
    pub fee: UsdValue,

    /// Caller-assigned id from the originally-submitted order, or `None`
    /// if the order was submitted without one.
    pub client_order_id: Option<ClientOrderId>,

    /// Identifier shared between the two `OrderFilled` events of a single
    /// order-book match (taker + maker). Strictly increasing across
    /// matches.
    ///
    /// `None` for trades executed before v0.15.0 — fill IDs were not assigned
    /// prior to that release.
    ///
    /// Equivalents at other venues:
    ///
    /// - BitMEX — `trdMatchID`, shared across the two executions of a
    ///   match; non-match `execType`s use an all-zeros placeholder:
    ///   <https://support.bitmex.com/hc/en-gb/articles/6205689858077--execution-field-definitions>
    /// - Hyperliquid — `tid`, a 50-bit hash of `(buyer_oid, seller_oid)`
    ///   that both sides of a match emit:
    ///   <https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/websocket/subscriptions>
    /// - Binance USD-M Futures — trade `id` on `GET /fapi/v1/userTrades`;
    ///   each side sees the same `id` per match:
    ///   <https://developers.binance.com/docs/derivatives/usds-margined-futures/trade/rest-api/Account-Trade-List>
    pub fill_id: Option<FillId>,

    /// `Some(true)` for the maker side of a match, `Some(false)` for the
    /// taker side. Each order-book match emits two `OrderFilled` events
    /// sharing one `fill_id` — exactly one with `is_maker = Some(true)`
    /// and one with `is_maker = Some(false)`.
    ///
    /// `None` for trades executed before v0.16.0 — the maker/taker flag was not
    /// recorded prior to that release.
    pub is_maker: Option<bool>,
}

/// Event indicating a user has been liquidated in a specific pair.
///
/// Emitted once per pair closed during liquidation. The market-order portion's
/// PnL is captured by `OrderFilled` events; this event reports the ADL
/// (off-book) portion.
#[grug::event("liquidated")]
#[grug::derive(Serde)]
pub struct Liquidated {
    pub user: Addr,
    pub pair_id: PairId,

    /// Size closed via ADL (zero if fully filled on book).
    pub adl_size: Quantity,

    /// Bankruptcy price used for ADL fills, or `None` if no ADL happened.
    pub adl_price: Option<UsdPrice>,

    /// PnL realized by the liquidated user from ADL fills, accumulated
    /// across all counter-party fills for this pair. Reports the closing
    /// PnL on each ADL fill (price movement on the closed portion).
    /// Zero if no ADL happened. ADL fills incur no trading fees.
    ///
    /// Prior to v0.17.0 (exclusive) this field also bundled the funding
    /// settled on the user's position immediately before each ADL fill.
    /// From v0.17.0 (inclusive) onward, that funding component is
    /// reported separately as `adl_realized_funding` and is no longer
    /// included here.
    pub adl_realized_pnl: UsdValue,

    /// Funding settled on the liquidated user's position immediately
    /// before each ADL fill, accumulated across all counter-party fills
    /// for this pair. Zero if no ADL happened or if no funding had
    /// accrued.
    ///
    /// `None` for liquidations executed before v0.17.0 — funding was
    /// reported as part of `adl_realized_pnl` prior to that release.
    pub adl_realized_funding: Option<UsdValue>,
}

/// Event indicating a counter-party's position was reduced during ADL.
///
/// Emitted for each counter-party hit during a liquidation's ADL step.
#[grug::event("deleveraged")]
#[grug::derive(Serde)]
pub struct Deleveraged {
    pub user: Addr,
    pub pair_id: PairId,

    /// Size closed (sign matches the reduction to the counter-party's position).
    pub closing_size: Quantity,

    /// Fill price (the liquidated user's bankruptcy price).
    pub fill_price: UsdPrice,

    /// PnL realized by the counter-party from this ADL fill from price
    /// movement on the closed portion. ADL fills incur no trading fees.
    ///
    /// Prior to v0.17.0 (exclusive) this field also bundled the funding
    /// settled on the counter-party's pre-existing position immediately
    /// before the fill. From v0.17.0 (inclusive) onward, that funding
    /// component is reported separately as `realized_funding` and is no
    /// longer included here.
    pub realized_pnl: UsdValue,

    /// Funding settled on the counter-party's pre-existing position
    /// immediately before this ADL fill is applied. Zero if the
    /// counter-party had no funding accrued in this pair.
    ///
    /// `None` for ADL fills executed before v0.17.0 — funding was
    /// reported as part of `realized_pnl` prior to that release.
    pub realized_funding: Option<UsdValue>,
}

/// Event indicating the insurance fund absorbed bad debt from a liquidation.
#[grug::event("bad_debt_covered")]
#[grug::derive(Serde)]
pub struct BadDebtCovered {
    pub liquidated_user: Addr,
    pub amount: UsdValue,
    pub insurance_fund_remaining: UsdValue,
}

/// Event emitted when referral commissions are distributed for a fee-paying user.
///
/// `commissions[0]` is the payer (referee), `commissions[1]` is the first referrer,
/// and so on up the chain. Zero entries indicate no commission at that level.
#[grug::event("fee_distributed")]
#[grug::derive(Serde)]
pub struct FeeDistributed {
    /// User index of the fee payer.
    pub payer: UserIndex,

    /// Address of the fee payer.
    pub payer_addr: Addr,

    /// Protocol treasury portion of the fee.
    pub protocol_fee: UsdValue,

    /// Vault portion of the fee.
    pub vault_fee: UsdValue,

    /// Commission amounts per chain level: [payer, 1st referrer, 2nd, ...].
    pub commissions: Vec<UsdValue>,
}

/// Event indicating a referral relationship has been registered.
#[grug::event("referral_set")]
#[grug::derive(Serde)]
pub struct ReferralSet {
    pub referrer: UserIndex,
    pub referee: UserIndex,
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {super::*, std::collections::BTreeMap};

    #[test]
    fn resolve_empty_tiers() {
        let schedule = RateSchedule {
            base: Dimensionless::new_permille(1),
            tiers: BTreeMap::new(),
        };
        assert_eq!(
            schedule.resolve(UsdValue::new_int(1_000_000)),
            schedule.base
        );
    }

    #[test]
    fn resolve_below_all_thresholds() {
        let schedule = RateSchedule {
            base: Dimensionless::new_permille(1),
            tiers: BTreeMap::from([
                (UsdValue::new_int(100_000), Dimensionless::new_raw(800)),
                (UsdValue::new_int(1_000_000), Dimensionless::new_raw(500)),
            ]),
        };
        assert_eq!(schedule.resolve(UsdValue::new_int(50_000)), schedule.base);
    }

    #[test]
    fn resolve_between_thresholds() {
        let tier1_rate = Dimensionless::new_raw(800);
        let schedule = RateSchedule {
            base: Dimensionless::new_permille(1),
            tiers: BTreeMap::from([
                (UsdValue::new_int(100_000), tier1_rate),
                (UsdValue::new_int(1_000_000), Dimensionless::new_raw(500)),
            ]),
        };
        assert_eq!(schedule.resolve(UsdValue::new_int(500_000)), tier1_rate);
    }

    #[test]
    fn resolve_above_all_thresholds() {
        let top_rate = Dimensionless::new_raw(500);
        let schedule = RateSchedule {
            base: Dimensionless::new_permille(1),
            tiers: BTreeMap::from([
                (UsdValue::new_int(100_000), Dimensionless::new_raw(800)),
                (UsdValue::new_int(1_000_000), top_rate),
            ]),
        };
        assert_eq!(schedule.resolve(UsdValue::new_int(5_000_000)), top_rate);
    }

    #[test]
    fn resolve_exactly_at_threshold() {
        let tier1_rate = Dimensionless::new_raw(800);
        let schedule = RateSchedule {
            base: Dimensionless::new_permille(1),
            tiers: BTreeMap::from([
                (UsdValue::new_int(100_000), tier1_rate),
                (UsdValue::new_int(1_000_000), Dimensionless::new_raw(500)),
            ]),
        };
        assert_eq!(schedule.resolve(UsdValue::new_int(100_000)), tier1_rate);
    }

    #[test]
    fn resolve_negative_tier_rate() {
        let base = Dimensionless::new_raw(-100); // -1 bps
        let tier1_rate = Dimensionless::new_raw(-200); // -2 bps
        let tier2_rate = Dimensionless::new_raw(-500); // -5 bps
        let schedule = RateSchedule {
            base,
            tiers: BTreeMap::from([
                (UsdValue::new_int(100_000), tier1_rate),
                (UsdValue::new_int(1_000_000), tier2_rate),
            ]),
        };

        assert_eq!(schedule.resolve(UsdValue::new_int(50_000)), base);
        assert_eq!(schedule.resolve(UsdValue::new_int(500_000)), tier1_rate);
        assert_eq!(schedule.resolve(UsdValue::new_int(5_000_000)), tier2_rate);
    }
}
