#[cfg(not(feature = "library"))]
use cw_std::entry_point;
use {
    anyhow::{bail, ensure},
    cw_std::{
        cw_derive, to_json, Addr, Binary, Coin, Coins, ImmutableCtx, Item, Message, MutableCtx,
        Querier, Response, StdResult, Uint128, Uint256,
    },
    std::cmp,
};

pub const CONFIG: Item<Config> = Item::new("config");

pub type InstantiateMsg = Config;

#[cw_derive(serde)]
pub enum ExecuteMsg {
    /// Add liquidity to the pool and mint share tokens.
    /// Must send non-zero amount of the the pool's two denoms.
    ProvideLiquidity {
        minimum_receive: Option<Uint128>,
    },
    /// Burn share tokens and withdraw liquidity.
    /// Must send non-zero amount of the pool's share token.
    WithdrawLiquidity {
        minimum_receive: Option<Coins>,
    },
    /// Make a swap.
    /// Must send non-zero amount of one of the pool's two denoms.
    Swap {
        minimum_receive: Option<Uint128>,
    },
}

#[cw_derive(serde)]
pub enum QueryMsg {
    /// The contract's configuration.
    /// Returns: Config
    Config {},
    /// Compute the output amount when offering to swap a given amount.
    /// Returns: Coin
    Simulate {
        offer: Coin,
    },
    /// Compute the input amount in order for a swap to output the given amount.
    /// Returns: Coin
    ReverseSimulate {
        ask: Coin,
    },
}

/// The AMM pool's configuration.
#[cw_derive(serde, borsh)]
pub struct Config {
    /// Address of the bank contract.
    /// We need to call the bank contract to mint or burn the share token.
    pub bank: Addr,
    /// Denomination of the pool's first token.
    pub denom1: String,
    /// Denomination of the pool's second token.
    pub denom2: String,
}

/// Return the denomination of the token that represents ownership shares of the
/// pool's liquidity.
pub fn share_token_denom(contract: &Addr) -> String {
    format!("amm/{contract}")
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> StdResult<Response> {
    CONFIG.save(ctx.store, &msg)?;

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn receive(_: MutableCtx) -> anyhow::Result<Response> {
    bail!("do not send tokens directly to this contract");
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(ctx: MutableCtx, msg: ExecuteMsg) -> anyhow::Result<Response> {
    match msg {
        ExecuteMsg::ProvideLiquidity {
            minimum_receive,
        } => provide_liquidity(ctx, minimum_receive),
        ExecuteMsg::WithdrawLiquidity {
            minimum_receive,
        } => withdraw_liquidity(ctx, minimum_receive),
        ExecuteMsg::Swap {
            minimum_receive,
        } => swap(ctx, minimum_receive),
    }
}

pub fn provide_liquidity(
    ctx: MutableCtx,
    minimum_receive: Option<Uint128>,
) -> anyhow::Result<Response> {
    // check the token(s) sent
    ensure!(ctx.funds.len() == 2, "must deposit exactly 2 coins, got: {}", ctx.funds.len());
    let cfg = CONFIG.load(ctx.store)?;
    let amount1 = ctx.funds.amount_of(&cfg.denom1);
    let amount2 = ctx.funds.amount_of(&cfg.denom2);
    ensure!(!amount1.is_zero(), "must deposit a non-zero amount of {}", cfg.denom1);
    ensure!(!amount2.is_zero(), "must deposit a non-zero amount of {}", cfg.denom2);

    // compute how many share tokens to mint
    let share_denom = share_token_denom(&ctx.contract);
    let total_shares = ctx.query_supply(share_denom.clone())?;
    let shares_to_mint = if total_shares.is_zero() {
        // this is the initial liquidity provision. in this case we define the
        // amount of shares to mint as geometric_mean(amount1, amount2)
        Uint256::from(amount1).checked_mul(Uint256::from(amount2))?
            .integer_sqrt().try_into()?

        // NOTE: for production use, the contract should permanently lockup a
        // small amount of share tokens, to avoid total shares being reduced to
        // zero completely, which can be used in some numerical attacks.
        //
        // see:
        // https://github.com/Uniswap/v2-core/blob/v1.0.1/contracts/UniswapV2Pair.sol#L121
        // https://github.com/astroport-fi/astroport-core/blob/v3.11.0/contracts/pair/src/contract.rs#L380
        //
        // however, since this is just a demo contract, we don't bother with this.
    } else {
        // NOTE: these depths are *after* receiving the deposits, so when
        // computing the share mint amount we need to subtract the deposit.
        let depth1 = ctx.query_balance(ctx.contract.clone(), cfg.denom1.clone())?;
        let depth2 = ctx.query_balance(ctx.contract.clone(), cfg.denom2.clone())?;
        cmp::min(
            amount1.checked_multiply_ratio(total_shares, depth1.checked_sub(amount1)?)?,
            amount2.checked_multiply_ratio(total_shares, depth2.checked_sub(amount2)?)?,
        )
    };

    // check slippage tolerance
    if let Some(min) = minimum_receive {
        ensure!(shares_to_mint >= min, "too much slippage: {shares_to_mint} < {min}");
    }

    Ok(Response::new()
        .add_attribute("method", "provide_liquidity")
        .add_attribute("funds_received", ctx.funds)
        .add_attribute("shares_minted", shares_to_mint)
        .add_message(Message::Execute {
            contract: cfg.bank,
            msg: to_json(&cw_bank::ExecuteMsg::Mint {
                to:     ctx.sender,
                denom:  share_denom,
                amount: shares_to_mint,
            })?,
            funds: Coins::new_empty(),
        }))
}

pub fn withdraw_liquidity(
    ctx: MutableCtx,
    minimum_receive: Option<Coins>,
) -> anyhow::Result<Response> {
    // check the token(s) sent
    ensure!(ctx.funds.len() == 1, "must deposit exactly 1 coin, got: {}", ctx.funds.len());
    let share_denom = share_token_denom(&ctx.contract);
    let shares_to_burn = ctx.funds.amount_of(&share_denom);
    ensure!(!shares_to_burn.is_zero(), "must deposit a non-zero amount of share token");

    // compute how many of the two tokens to refund the user
    let cfg = CONFIG.load(ctx.store)?;
    let total_shares = ctx.query_supply(share_denom.clone())?;
    let depth1 = ctx.query_balance(ctx.contract.clone(), cfg.denom1.clone())?;
    let amount1 = depth1.checked_multiply_ratio(shares_to_burn, total_shares)?;
    let depth2 = ctx.query_balance(ctx.contract.clone(), cfg.denom2.clone())?;
    let amount2 = depth2.checked_multiply_ratio(shares_to_burn, total_shares)?;

    // check slippage tolerance
    if let Some(min) = minimum_receive {
        let min1 = min.amount_of(&cfg.denom1);
        ensure!(amount1 >= min1, "too much slippage for {}: {amount1} < {min1}", cfg.denom1);
        let min2 = min.amount_of(&cfg.denom2);
        ensure!(amount2 >= min2, "too much slippage for {}: {amount2} < {min2}", cfg.denom2);
    }

    let mut refunds = Coins::new_empty();
    refunds.increase_amount(&cfg.denom1, amount1)?;
    refunds.increase_amount(&cfg.denom2, amount2)?;

    Ok(Response::new()
        .add_attribute("method", "withdraw_liquidity")
        .add_attribute("shares_burned", shares_to_burn)
        .add_attribute("funds_returned", &refunds)
        .add_message(Message::Execute {
            contract: cfg.bank.clone(),
            msg: to_json(&cw_bank::ExecuteMsg::Burn {
                from:   ctx.contract,
                denom:  share_denom,
                amount: shares_to_burn,
            })?,
            funds: Coins::new_empty(),
        })
        .add_message(Message::Transfer {
            to:    ctx.sender,
            coins: refunds,
        }))
}

pub fn swap(ctx: MutableCtx, minimum_receive: Option<Uint128>) -> anyhow::Result<Response> {
    // check the token(s) sent
    let cfg = CONFIG.load(ctx.store)?;
    let offer = ctx.funds.one_coin()?;
    let ask_denom = if *offer.denom == cfg.denom1 {
        cfg.denom2
    } else if *offer.denom == cfg.denom2 {
        cfg.denom1
    } else {
        bail!("must offer either {} or {}, got {}", cfg.denom1, cfg.denom2, offer.denom);
    };

    // compute the swap output amount
    // for the offer denom's depth, we need to subtract the deposited amount
    let offer_depth = ctx
        .query_balance(ctx.contract.clone(), offer.denom.clone())?
        .checked_sub(*offer.amount)?;
    let ask_depth = ctx.query_balance(ctx.contract.clone(), ask_denom.clone())?;
    let ask_amount = compute_swap_output(*offer.amount, offer_depth, ask_depth)?;

    // check slippage tolerance
    if let Some(min) = minimum_receive {
        ensure!(ask_amount >= min, "too much slippage: {ask_amount} < {min}");
    }

    Ok(Response::new()
        .add_attribute("method", "swap")
        .add_attribute("offer_denom", offer.denom)
        .add_attribute("offer_amount", offer.amount)
        .add_attribute("offer_depth", offer_depth)
        .add_attribute("ask_denom", &ask_denom)
        .add_attribute("ask_amount", ask_amount)
        .add_attribute("ask_depth", ask_depth)
        .add_message(Message::Transfer {
            to: ctx.sender,
            coins: Coin::new(ask_denom, ask_amount).into(),
        }))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> anyhow::Result<Binary> {
    match msg {
        QueryMsg::Config {} => to_json(&query_config(ctx)?),
        QueryMsg::Simulate {
            offer,
        } => to_json(&query_simulate(ctx, offer)?),
        QueryMsg::ReverseSimulate {
            ask,
        } => to_json(&query_reverse_simulate(ctx, ask)?),
    }
    .map_err(Into::into)
}

pub fn query_config(ctx: ImmutableCtx) -> StdResult<Config> {
    CONFIG.load(ctx.store)
}

pub fn query_simulate(ctx: ImmutableCtx, offer: Coin) -> anyhow::Result<Coin> {
    let cfg = CONFIG.load(ctx.store)?;
    let ask_denom = if offer.denom == cfg.denom1 {
        cfg.denom2
    } else if offer.denom == cfg.denom2 {
        cfg.denom1
    } else {
        bail!("must offer either {} or {}, got {}", cfg.denom1, cfg.denom2, offer.denom);
    };

    let offer_depth = ctx.query_balance(ctx.contract.clone(), offer.denom.clone())?;
    let ask_depth = ctx.query_balance(ctx.contract.clone(), ask_denom.clone())?;
    let ask_amount = compute_swap_output(offer.amount, offer_depth, ask_depth)?;

    Ok(Coin::new(ask_denom, ask_amount))
}

pub fn query_reverse_simulate(ctx: ImmutableCtx, ask: Coin) -> anyhow::Result<Coin> {
    let cfg = CONFIG.load(ctx.store)?;
    let offer_denom = if ask.denom == cfg.denom1 {
        cfg.denom2
    } else if ask.denom == cfg.denom2 {
        cfg.denom1
    } else {
        bail!("must ask either {} or {}, got {}", cfg.denom1, cfg.denom2, ask.denom);
    };

    let offer_depth = ctx.query_balance(ctx.contract.clone(), offer_denom.clone())?;
    let ask_depth = ctx.query_balance(ctx.contract.clone(), ask.denom.clone())?;
    let offer_amount = compute_swap_input(offer_depth, ask_depth, ask.amount)?;

    Ok(Coin::new(offer_denom, offer_amount))
}

// offer_depth * ask_depth = (offer_depth + offer_amount) * (ask_depth - ask_amount)
// => ask_amount = ask_depth - offer_depth * ask_depth / (offer_depth + offer_amount)
fn compute_swap_output(
    offer_amount: Uint128,
    offer_depth: Uint128,
    ask_depth: Uint128,
) -> StdResult<Uint128> {
    let rhs = ask_depth.checked_multiply_ratio(
        offer_depth,
        offer_depth.checked_add(offer_amount)?,
    )?;
    ask_depth.checked_sub(rhs)
}

// offer_amount = offer_depth * ask_depth / (ask_depth - ask_amount) - offer_depth
// TODO: should be be rounding up the division? currently it's rounding down.
fn compute_swap_input(
    offer_depth: Uint128,
    ask_depth: Uint128,
    ask_amount: Uint128,
) -> StdResult<Uint128> {
    let lhs = offer_depth.checked_multiply_ratio(
        ask_depth,
        ask_depth.checked_sub(ask_amount)?,
    )?;
    lhs.checked_sub(offer_depth)
}
