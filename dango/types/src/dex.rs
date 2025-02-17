use {
    anyhow::ensure,
    grug::{
        Addr, Coin, Coins, Denom, Int, MultiplyFraction, MultiplyRatio, NumberConst, Part,
        PrimaryKey, RawKey, StdError, StdResult, Udec128, Uint128,
    },
    std::{
        collections::{BTreeMap, BTreeSet},
        fmt::Display,
        str::FromStr,
        sync::LazyLock,
    },
};

/// The namespace used for dex.
///
/// E.g.,
///
/// - `dex/eth`
/// - `dex/usdc`
pub static NAMESPACE: LazyLock<Part> = LazyLock::new(|| Part::new_unchecked("dex"));

/// The subnamespace used for lp tokens for the passive pools.
///
/// E.g.,
///
/// - `dex/lp/ethusdc`
/// - `dex/lp/btcusdc`
pub static LP_NAMESPACE: LazyLock<Part> = LazyLock::new(|| Part::new_unchecked("lp"));

// ----------------------------------- types -----------------------------------

/// Numerical identifier of an order.
///
/// For SELL orders, we count order IDs from 0 up; for BUY orders, from `u64::MAX`
/// down.
///
/// As such, given our contract storage layout, between two orders of the same
/// price, the older one is matched first. This follows the principle of
/// **price-time priority**.
///
/// Note that this assumes `order_id` never exceeds `u64::MAX / 2`, which is a
/// safe assumption. If we accept 1 million orders per second, it would take
/// ~300,000 years to reach `u64::MAX / 2`.
pub type OrderId = u64;

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
                "invalid order direction! must be 0|1",
            )),
        }
    }
}

#[grug::derive(Serde)]
pub struct Swap {
    pub base_denom: Denom,
    pub quote_denom: Denom,
    pub direction: Direction,
    pub amount: Uint128,
    pub slippage: Option<SlippageControl>,
}

#[grug::derive(Serde)]
pub enum SlippageControl {
    /// Minimum amount out. Transaction will fail if the amount out is less than the
    /// specified amount.
    MinimumOut(Uint128),
    /// Maximum amount in. Transaction will fail if the amount in is greater than the
    /// specified amount.
    MaximumIn(Uint128),
    /// Price limit. Transaction will fail if the execution price is greater than the
    /// specified price for a BUY order, or less than the specified price for a SELL order.
    PriceLimit(Udec128),
}

#[grug::derive(Serde)]
pub struct Pair {
    pub base_denom: Denom,
    pub quote_denom: Denom,
}

#[grug::derive(Serde, Borsh)]
pub struct PairParams {
    // TODO: add:
    // - fee rate (either here or as a global parameter)
    // - tick size (necessary or not?)
    // - minimum order size
    // - params for the passive liquidity pool
}

#[grug::derive(Serde)]
pub struct PairUpdate {
    pub base_denom: Denom,
    pub quote_denom: Denom,
    pub params: PairParams,
}

#[grug::derive(Serde)]
pub struct OrderResponse {
    pub user: Addr,
    pub base_denom: Denom,
    pub quote_denom: Denom,
    pub direction: Direction,
    pub price: Udec128,
    pub amount: Uint128,
    pub remaining: Uint128,
}

#[grug::derive(Serde)]
pub struct OrdersByPairResponse {
    pub user: Addr,
    pub direction: Direction,
    pub price: Udec128,
    pub amount: Uint128,
    pub remaining: Uint128,
}

#[grug::derive(Serde)]
pub struct OrdersByUserResponse {
    pub base_denom: Denom,
    pub quote_denom: Denom,
    pub direction: Direction,
    pub price: Udec128,
    pub amount: Uint128,
    pub remaining: Uint128,
}

// --------------------------------- messages ----------------------------------

#[grug::derive(Serde)]
pub struct InstantiateMsg {
    pub pairs: Vec<PairUpdate>,
}

#[grug::derive(Serde)]
pub enum ExecuteMsg {
    /// Create new, or modify the parametes of existing, trading pairs.
    ///
    /// Can only be called by the chain owner.
    BatchUpdatePairs(Vec<PairUpdate>),
    /// Create a new passive pool for a pair.
    ///
    /// Can only be called by the chain owner.
    CreatePassivePool {
        base_denom: Denom,
        quote_denom: Denom,
        curve_type: CurveInvariant,
        lp_denom: Denom,
        swap_fee: Udec128,
    },
    /// Submit a new order.
    ///
    /// - For SELL orders, sender must attach `base_denom` of `amount` amount.
    ///
    /// - For BUY orders, sender must attach `quote_denom` of the amount
    ///   calculated as:
    ///
    ///   ```plain
    ///   ceil(amount * price)
    ///   ```
    SubmitOrder {
        base_denom: Denom,
        quote_denom: Denom,
        direction: Direction,
        amount: Uint128,
        price: Udec128,
    },
    /// Cancel one or more orders by IDs.
    CancelOrders {
        order_ids: BTreeSet<OrderId>,
    },
    /// Perform the swaps
    BatchSwap {
        swaps: Vec<Swap>,
    },
    // Provide passive liquidity to a pair.
    ProvideLiquidity {
        lp_denom: Denom,
    },
    // Withdraw passive liquidity from a pair.
    WithdrawLiquidity {},
}

#[grug::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    /// Query the parameters of a single trading pair.
    #[returns(PairParams)]
    Pair {
        base_denom: Denom,
        quote_denom: Denom,
    },
    /// Enumerate all trading pairs and their parameters.
    #[returns(Vec<PairUpdate>)]
    Pairs {
        start_after: Option<Pair>,
        limit: Option<u32>,
    },
    /// Query a single active order by ID.
    #[returns(OrderResponse)]
    Order { order_id: OrderId },
    /// Enumerate active orders across all pairs and from users.
    #[returns(BTreeMap<OrderId, OrderResponse>)]
    Orders {
        start_after: Option<OrderId>,
        limit: Option<u32>,
    },
    /// Enumerate active orders in a single pair from all users.
    #[returns(BTreeMap<OrderId, OrdersByPairResponse>)]
    OrdersByPair {
        base_denom: Denom,
        quote_denom: Denom,
        start_after: Option<OrderId>,
        limit: Option<u32>,
    },
    /// Enumerate active orders from a single user across all pairs.
    #[returns(BTreeMap<OrderId, OrdersByUserResponse>)]
    OrdersByUser {
        user: Addr,
        start_after: Option<OrderId>,
        limit: Option<u32>,
    },
    /// Query the passive pool for a pair.
    #[returns(Pool)]
    PassivePool { lp_denom: Denom },

    #[returns(Denom)]
    LpDenom {
        base_denom: Denom,
        quote_denom: Denom,
    },
}

// ---------------------------------- events -----------------------------------

#[grug::derive(Serde)]
#[grug::event("pair_updated")]
pub struct PairUpdated {
    pub base_denom: Denom,
    pub quote_denom: Denom,
}

#[grug::derive(Serde)]
#[grug::event("order_submitted")]
pub struct OrderSubmitted {
    pub order_id: OrderId,
    pub user: Addr,
    pub base_denom: Denom,
    pub quote_denom: Denom,
    pub direction: Direction,
    pub price: Udec128,
    pub amount: Uint128,
    pub deposit: Coin,
}

#[grug::derive(Serde)]
#[grug::event("order_canceled")]
pub struct OrderCanceled {
    pub order_id: OrderId,
    pub remaining: Uint128,
    pub refund: Coin,
}

#[grug::derive(Serde)]
#[grug::event("orders_matched")]
pub struct OrdersMatched {
    pub base_denom: Denom,
    pub quote_denom: Denom,
    pub clearing_price: Udec128,
    pub volume: Uint128,
}

#[grug::derive(Serde)]
#[grug::event("order_filled")]
pub struct OrderFilled {
    pub order_id: OrderId,
    /// The price at which the order was executed.
    pub clearing_price: Udec128,
    /// The amount (measured in base asset) that was filled.
    pub filled: Uint128,
    /// The amount of coins returned to the user.
    pub refund: Coins,
    /// The amount of protocol fee collected.
    pub fee: Option<Coin>,
    /// Whether the order was _completed_ filled and cleared from the book.
    pub cleared: bool,
}

#[grug::derive(Serde, Borsh)]
#[non_exhaustive]
pub enum CurveInvariant {
    Xyk,
}

impl CurveInvariant {
    pub fn solve_amount_out(
        &self,
        offer: Coin,
        ask_denom: &Denom,
        swap_fee: Udec128,
        reserves: &Coins,
    ) -> anyhow::Result<Uint128> {
        ensure!(
            reserves.has(&offer.denom) && reserves.has(ask_denom),
            "invalid reserves"
        );
        match self {
            CurveInvariant::Xyk => {
                let a = reserves.amount_of(&offer.denom);
                let b = reserves.amount_of(ask_denom);

                // Solve A * B = (A + offer.amount) * (B - amount_out) for amount_out
                // => amount_out = B - (A * B) / (A + offer.amount)
                // Round so that user takes the loss
                let amount_out =
                    b - Int::ONE.checked_multiply_ratio_ceil(a * b, a + offer.amount)?;

                // Apply swap fee. Round so that user takes the loss
                let amount_out = amount_out.checked_mul_dec_floor(Udec128::ONE - swap_fee)?;

                Ok(amount_out)
            },
        }
    }

    pub fn solve_amount_in(
        &self,
        ask: Coin,
        offer_denom: &Denom,
        swap_fee: Udec128,
        reserves: &Coins,
    ) -> anyhow::Result<Uint128> {
        ensure!(
            reserves.has(offer_denom) && reserves.has(&ask.denom),
            "invalid reserves"
        );
        ensure!(
            reserves.amount_of(&ask.denom) > ask.amount,
            "insufficient liquidity"
        );
        match self {
            CurveInvariant::Xyk => {
                let a = reserves.amount_of(offer_denom);
                let b = reserves.amount_of(&ask.denom);

                // Apply swap fee. In SwapExactIn we multiply ask by (1 - fee) to get the
                // offer amount after fees. So in this case we need to divide ask by (1 - fee)
                // to get the ask amount after fees. Round so that user takes the loss
                let ask_amount_after_fee =
                    ask.amount.checked_div_dec_ceil(Udec128::ONE - swap_fee)?;

                // Solve A * B = (A + amount_in) * (B - ask.amount) for amount_in
                // => amount_in = (A * B) / (B - ask.amount) - A
                // Round so that user takes the loss
                let amount_in =
                    Int::ONE.checked_multiply_ratio_ceil(a * b, b - ask_amount_after_fee)? - a;

                Ok(amount_in)
            },
        }
    }
}

impl Display for CurveInvariant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            CurveInvariant::Xyk => "xyk",
        };
        write!(f, "{}", s)
    }
}

impl FromStr for CurveInvariant {
    type Err = StdError;

    fn from_str(s: &str) -> StdResult<Self> {
        match s {
            "xyk" => Ok(CurveInvariant::Xyk),
            _ => Err(StdError::deserialize::<Self, _>(
                "str",
                "invalid curve type",
            )),
        }
    }
}

#[grug::derive(Serde, Borsh)]
pub struct Pool {
    pub base_denom: Denom,
    pub quote_denom: Denom,
    pub curve_type: CurveInvariant,
    pub reserves: Coins,
    pub swap_fee: Udec128,
}

impl Pool {
    /// Swap an exact amount of an input token for an output token in the pool.
    /// Mutates the pool reserves.
    ///
    /// Returns the amount of the output token received.
    pub fn swap_exact_amount_in(&mut self, offer: Coin, ask_denom: &Denom) -> anyhow::Result<Coin> {
        // Ensure the offer and ask  denom is valid
        ensure!(
            self.reserves.has(&offer.denom) && self.reserves.has(ask_denom),
            "invalid reserves"
        );

        // Calculate the out amount
        let out_amount = self.curve_type.solve_amount_out(
            offer.clone(),
            ask_denom,
            self.swap_fee,
            &self.reserves,
        )?;
        let ask = Coin {
            denom: ask_denom.clone(),
            amount: out_amount,
        };

        // Update the pool reserves
        self.reserves.insert(offer)?;
        self.reserves.deduct(ask.clone())?;

        Ok(ask)
    }

    pub fn swap_exact_amount_out(
        &mut self,
        ask: Coin,
        offer_denom: &Denom,
    ) -> anyhow::Result<Coin> {
        println!("swap_exact_amount_out {:?}", ask);
        // Ensure the offer and ask  denom is valid
        ensure!(
            self.reserves.has(offer_denom) && self.reserves.has(&ask.denom),
            "invalid reserves"
        );

        // Calculate the in amount
        let in_amount = self.curve_type.solve_amount_in(
            ask.clone(),
            offer_denom,
            self.swap_fee,
            &self.reserves,
        )?;
        let offer = Coin {
            denom: offer_denom.clone(),
            amount: in_amount,
        };

        println!("offer {:?}", offer);
        println!("reserves {:?}", self.reserves);
        // Update the pool reserves
        self.reserves.insert(offer.clone())?;
        self.reserves.deduct(ask)?;

        Ok(offer)
    }
}
