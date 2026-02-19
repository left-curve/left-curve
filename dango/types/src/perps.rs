use {
    crate::{BaseAmount, HumanAmount, Ratio, UsdPrice, UsdValue},
    grug::{Addr, Denom, Duration, Part, Timestamp},
    std::{
        collections::{BTreeMap, BTreeSet},
        sync::LazyLock,
    },
};

// --------------------------------- Constants ---------------------------------

/// Namespace for tokens minted by the perps contract.
pub static NAMESPACE: LazyLock<Part> = LazyLock::new(|| Part::new_unchecked("perps"));

// ----------------------------------- Types -----------------------------------

/// Identifier of a trading pair. It should be a string that looks like e.g. "BTCUSD-PERP".
pub type PairId = String;

/// Identifies a resting limit order.
pub type OrderId = u64;

#[grug::derive(Serde)]
pub enum OrderKind {
    /// Trade at the current marginal price, plus/minus a maximum slippage.
    ///
    /// Marginal price is the price quoted by the counterparty vault for an order
    /// of infinitesimal size. It's calculated based on the oracle price and
    /// the current skew (the differencce bewteen long and short OI).
    Market { max_slippage: Ratio<UsdPrice> },

    /// Trade at the specified limit price.
    Limit { limit_price: UsdPrice },
}

/// Global parameters that concerns the counterparty vault and all trading pairs.
#[grug::derive(Serde, Borsh)]
pub struct Param {
    /// Denomination of the asset used to settle perpetual futures contracts.
    pub settlement_currency: Denom,

    /// Once a request to withdraw liquidity from the counterparty vault has been
    /// submitted, the waiting time that must elapsed before the funds are released
    /// to the liquidity provider.
    pub vault_cooldown_period: Duration,

    /// Maximum number of unlock requests a single user may have.
    pub max_unlocks: u32,

    /// Maximum number of resting limit order a single user may have across all
    /// trading pairs.
    pub max_open_orders: u32,

    /// Trading fee as a fraction of an order's notional value, deducted from
    /// the user's margin and transferred to the vault on every fill.
    ///
    /// fee = ceil(|fill_size| * exec_price * trading_fee_rate / settlement_currency_price)
    ///
    /// E.g. with trading_fee_rate = 0.05%, for a fill of notional value $100,000,
    /// and price of the settlement currency is $0.95 per USDT:
    ///
    /// fee = ceil($100,000 * 0.05% / $0.95 per USDT)
    ///     = 52.631579 USDT
    pub trading_fee_rate: Ratio<UsdValue>,

    /// Fee paid to the vault as a fraction of the total notional value of
    /// positions being liquidated, capped at the user's remaining margin after
    /// position closure.
    ///
    /// fee = min(
    ///   ceil(|position_size| * oracle_price * liquidation_fee_rate / settlement_currency_price),
    ///   user_remaining_margin
    /// )
    pub liquidation_fee_rate: Ratio<UsdValue>,

    /// Ratio of vault equity to total open notional below which ADL is enabled.
    ///
    /// When vault_equity < adl_trigger_ratio * total_open_notional, the `deleverage`
    /// execute method becomes callable by whitelisted addresses.
    pub adl_trigger_ratio: Ratio<UsdValue>,

    /// Accounts who are authorized to deleverage users when ADL trigger condition
    /// is met.
    pub adl_operators: BTreeSet<Addr>,
}

/// Parameters that apply to an individual trading pair.
#[grug::derive(Serde, Borsh)]
pub struct PairParam {
    /// A scaling factor that determines how greatly an imbalance in long/short
    /// open interests (the "skew") should affect the price quoted by the vault.
    /// The greater the value of the scaling factor, the less the effect.
    pub skew_scale: Ratio<HumanAmount, Ratio<UsdPrice>>,

    /// The maximum extent to which skew can affect the quote price.
    ///
    /// That is, the execution price of an order is clamped to the range:
    /// oracle_price * (1 + [-max_abs_premium, max_abs_premium]).
    ///
    /// This prevents an exploit where a trader fabricates a big skew to obtain
    /// an unusually favorable pricing. See the [Mars Protocol hack](https://x.com/neutron_org/status/2014048218598838459).
    pub max_abs_premium: Ratio<UsdPrice>,

    /// The maximum allowed open interest for both long and short.
    /// I.e. the following must be satisfied:
    ///
    /// pair_state.long_oi <= max_abs_oi && pair_state.short_oi <= max_abs_oi
    ///
    /// This constraint does not apply to reduce-only orders.
    pub max_abs_oi: HumanAmount,

    /// Maximum absolute funding rate, as a fraction per day.
    ///
    /// That is, the daily funding rate is clamped to the range
    /// [-max_abs_funding_rate, max_abs_funding_rate].
    ///
    /// This prevents runaway rates from causing cascading liquidations and bad
    /// debt spirals during prolonged skew.
    pub max_abs_funding_rate: Ratio<UsdValue, Duration>,

    /// Maximum rate the funding rate may change, as a fraction per day.
    ///
    /// When |skew| >= skew_scale, the funding rate changes by this much per day.
    /// When skew == 0, the rate drifts back toward zero at this speed.
    pub max_funding_velocity: Ratio<UsdValue, Duration>,

    /// Minimum notional value for the opening portion of an order.
    /// This prevents the opening of dust positions.
    pub min_opening_size: UsdValue,

    /// Margin requirement when opening or increasing a position in this trading
    /// pair. E.g. 5% indicates a 1 / 5% = 20x maximum leverage.
    ///
    /// initial_margin = |position_size| * oracle_price * initial_margin_ratio
    pub initial_margin_ratio: Ratio<UsdValue>,

    /// Margin requirement for maintaining a position in this trading pair.
    ///
    /// Must be strictly less than `initial_margin_ratio`.
    ///
    /// When a user's equity falls below the sum of maintenance margins across
    /// all his positions, the user becomes eligible for liquidations.
    ///
    /// maintenance_margin = |position_size| * oracle_price * maintenance_margin_ratio
    pub maintenance_margin_ratio: Ratio<UsdValue>,
}

/// Global state that concerns the counterparty vault and all trading pairs.
#[grug::derive(Serde)]
pub struct State {
    /// The vault's collateral balance. Should be the sum of all user deposits,
    /// the vault's _realized_ PnL, and the share of trading fees earned by the vault.
    ///
    /// Note:
    ///
    /// - This doesn't equal the vault's _equity_, which on top of this also
    ///   includes the vault's _unrealized_ PnL.
    /// - This also doesn't equal the vault's token balance tracked by the bank
    ///   contract, which also includes unlocks that are pending cooldown.
    pub vault_margin: BaseAmount,

    /// Total supply of the vault's share token.
    pub vault_share_supply: BaseAmount,
}

/// State of an individual trading pair.
#[grug::derive(Serde, Borsh)]
pub struct PairState {
    /// The sum of the sizes of all long positions.
    pub long_oi: HumanAmount,

    /// The sum of the absolute value of the sizes of all short positions.
    pub short_oi: HumanAmount,

    /// Instantaneous funding rate (fraction per day) at the `last_funding_time`.
    ///
    /// Positive = longs pay shorts (and vault collects the net)
    /// Negative = shorts pay longs (and vault collects the net)
    ///
    /// The rate changes linearly over time according to the velocity model:
    ///   rate' = rate + velocity * elapsed_days
    pub funding_rate: Ratio<UsdValue, Duration>,

    /// Timestamp of the most recent funding accrual.
    pub last_funding_time: Timestamp,

    /// Cumulative funding per unit of position size, denominated in USD.
    ///
    /// This is an ever-increasing accumulator. To compute a position's accrued
    /// funding, take the difference between the current value and the position's
    /// `entry_funding_per_unit`.
    pub cumulative_funding_per_unit: Ratio<UsdValue, HumanAmount>,

    /// Sum of `position.size * position.entry_funding_per_unit` across all open
    /// positions for this pair.
    ///
    /// Used to compute the vault's unrealized funding without iterating over
    /// all positions:
    ///   vault_unrealized_funding = cumulative_funding_per_unit * skew - oi_weighted_entry_funding
    pub oi_weighted_entry_funding: UsdValue,

    /// Sum of `position.size * position.entry_price` across all open
    /// positions for this pair.
    ///
    /// Used to compute the vault's unrealized price PnL without iterating over
    /// all positions:
    ///   vault_unrealized_pnl = oi_weighted_entry_price - oracle_price * skew
    pub oi_weighted_entry_price: UsdValue,
}

/// State of a specific user.
#[grug::derive(Serde, Borsh)]
pub struct UserState {
    /// The user's vault withdrawals that are pending cooldown.
    pub unlocks: Vec<Unlock>, // TODO: use VecDeque?

    /// The user's open positions.
    pub positions: BTreeMap<PairId, Position>,

    /// Margin reserved for resting limit orders.
    pub reserved_margin: BaseAmount,

    /// Number of resting limit orders the user currently has on the book.
    pub open_order_count: u32,
}

/// A user's position in a specific trading pair.
#[grug::derive(Serde, Borsh)]
pub struct Position {
    /// The position's size. Position = long, negative = short.
    pub size: HumanAmount,

    /// The average price at which this position was entered.
    pub entry_price: UsdPrice,

    /// The value of `pair_state.cumulative_funding_per_unit` at the time when
    /// this position was last opened, modified, or funding settled.
    pub entry_funding_per_unit: Ratio<UsdValue, HumanAmount>,
}

/// A pending withdrawal of liquidity from the counterparty vault, awaiting the
/// cooldown period to elapse.
#[grug::derive(Serde, Borsh)]
pub struct Unlock {
    /// The amount of settlement currency to be released once cooldown completes.
    pub amount_to_release: BaseAmount,

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
    pub size: HumanAmount,
    pub reduce_only: bool,
    pub reserved_margin: BaseAmount,
}

// --------------------------------- Messages ----------------------------------

#[grug::derive(Serde)]
pub struct InstantiateMsg {
    // TODO
}

#[grug::derive(Serde)]
pub enum ExecuteMsg {
    /// Add liquidity to the counterparty vault.
    Deposit {
        /// Revert if less than this amount of shares is minted.
        min_shares_to_mint: Option<BaseAmount>,
    },

    /// Request to withdraw funds from the counterparty vault.
    ///
    /// The request will be fulfilled after the cooldown period has elapsed.
    Unlock {},

    /// Submit an order.
    SubmitOrder {
        pair_id: PairId,

        /// The amount of futures contract to buy or sell.
        /// Positive indicates buy, negative indicates sell.
        size: HumanAmount,

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
    CancelOrder { order_id: OrderId },

    /// Forcibly close all of a user's positions, if the user has less collateral
    /// than the maintenance margin required by his positions.
    Liquidate { user: Addr },

    /// Forcibly close all of a user's positions, even if the user has sufficient
    /// amount of collateral.
    ///
    /// This can only be called by whitelisted callers when the counterparty vault
    /// is in distress.
    Deleverage { user: Addr },
}

#[grug::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    /// Query the global parameters.
    #[returns(Param)]
    Param {},

    /// Query the pair-specific parameters of a single trading pair.
    #[returns(PairParam)]
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
    #[returns(PairState)]
    PairState { pair_id: PairId },

    /// Enumerate the pair-specific states of all trading pairs.
    #[returns(BTreeMap<PairId, PairState>)]
    PairStates {
        start_after: Option<PairId>,
        limit: Option<u32>,
    },

    /// Query the state of a single user.
    #[returns(UserState)]
    UserState { user: Addr },

    /// Enumerate the states of all users.
    #[returns(BTreeMap<Addr, UserState>)]
    UserStates {
        start_after: Option<Addr>,
        limit: Option<u32>,
    },
}

// ---------------------------------- Events -----------------------------------

// TODO
