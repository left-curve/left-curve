use anyhow::ensure;
#[cfg(not(feature = "library"))]
use cw_std::entry_point;
use {
    anyhow::bail,
    cw_std::{
        cw_serde, to_json, Addr, Binary, Coin, Coins, ExecuteCtx, InstantiateCtx, Item, Message,
        QueryCtx, ReceiveCtx, Response, StdResult, Uint128,
    },
    std::cmp,
};

pub const CONFIG: Item<Config> = Item::new("config");

pub type InstantiateMsg = Config;

#[cw_serde]
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

#[cw_serde]
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
#[cw_serde]
pub struct Config {
    /// Address of the bank contract.
    /// We need to call the bank contract to mint or burn the share token.
    pub bank:   Addr,
    /// Denomination of the pool's first token.
    pub denom1: String,
    /// Denomination of the pool's second token.
    pub denom2: String,
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(ctx: InstantiateCtx, msg: InstantiateMsg) -> StdResult<Response> {
    CONFIG.save(ctx.store, &msg)?;

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn receive(_: ReceiveCtx) -> anyhow::Result<Response> {
    bail!("do not send tokens directly to this contract");
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(ctx: ExecuteCtx, msg: ExecuteMsg) -> anyhow::Result<Response> {
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
    ctx: ExecuteCtx,
    minimum_receive: Option<Uint128>,
) -> anyhow::Result<Response> {
    ensure!(ctx.funds.len() == 2, "must send exactly 2 coins, got: {}", ctx.funds.len());

    let cfg = CONFIG.load(ctx.store)?;
    let amount1 = ctx.funds.amount_of(&cfg.denom1);
    let amount2 = ctx.funds.amount_of(&cfg.denom2);
    ensure!(!amount1.is_zero(), "must send a non-zero amount of {}", cfg.denom1);
    ensure!(!amount2.is_zero(), "must send a non-zero amount of {}", cfg.denom2);

    let share_denom = share_token_denom(&ctx.contract);
    let total_shares = ctx.query_supply(share_denom.clone())?;
    let shares_to_mint = if total_shares.is_zero() {
        // this is the initial liquidity provision. in this case we define the
        // amount of shares to mint as geometric_mean(amount1, amount2)
        amount1.checked_mul(amount2)?.integer_sqrt()

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
        let depth1 = ctx.query_balance(ctx.contract.clone(), cfg.denom1.clone())?;
        let depth2 = ctx.query_balance(ctx.contract.clone(), cfg.denom2.clone())?;
        cmp::min(
            amount1.checked_multiply_ratio(total_shares, depth1)?,
            amount2.checked_multiply_ratio(total_shares, depth2)?,
        )
    };

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
    ctx: ExecuteCtx,
    minimum_receive: Option<Coins>,
) -> anyhow::Result<Response> {
    ensure!(ctx.funds.len() == 1, "must send exactly 1 coin, got: {}", ctx.funds.len());

    let share_denom = share_token_denom(&ctx.contract);
    let shares_to_burn = ctx.funds.amount_of(&share_denom);
    ensure!(!shares_to_burn.is_zero(), "must send a non-zero amount of share token");

    let cfg = CONFIG.load(ctx.store)?;
    let total_shares = ctx.query_supply(share_denom.clone())?;

    let depth1 = ctx.query_balance(ctx.contract.clone(), cfg.denom1.clone())?;
    let amount1 = depth1.checked_multiply_ratio(shares_to_burn, total_shares)?;
    let depth2 = ctx.query_balance(ctx.contract.clone(), cfg.denom2.clone())?;
    let amount2 = depth2.checked_multiply_ratio(shares_to_burn, total_shares)?;

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

pub fn swap(ctx: ExecuteCtx, minimum_receive: Option<Uint128>) -> anyhow::Result<Response> {
    let cfg = CONFIG.load(ctx.store)?;
    let coin = ctx.funds.one_coin()?;
    let (offer_denom, ask_denom) = if *coin.denom == cfg.denom1 {
        (cfg.denom1, cfg.denom2)
    } else if *coin.denom == cfg.denom2 {
        (cfg.denom2, cfg.denom1)
    } else {
        bail!("must send either {} or {}, got {}", cfg.denom1, cfg.denom2, coin.denom);
    };

    let offer_depth = ctx.query_balance(ctx.contract.clone(), offer_denom.clone());
    let ask_depth = ctx.query_balance(ctx.contract.clone(), ask_denom.clone());

    // let return_amount =
    // TODO........

    Ok(Response::new())
}

pub fn share_token_denom(contract: &Addr) -> String {
    format!("amm/{contract}")
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(ctx: QueryCtx, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_json(&query_config(ctx)?),
        QueryMsg::Simulate {
            offer,
        } => to_json(&query_simulate(ctx, offer)?),
        QueryMsg::ReverseSimulate {
            ask,
        } => to_json(&query_reverse_simulate(ctx, ask)?),
    }
}

pub fn query_config(ctx: QueryCtx) -> StdResult<Config> {
    CONFIG.load(ctx.store)
}

pub fn query_simulate(ctx: QueryCtx, offer: Coin) -> StdResult<Coin> {
    todo!()
}

pub fn query_reverse_simulate(ctx: QueryCtx, ask: Coin) -> StdResult<Coin> {
    todo!()
}
