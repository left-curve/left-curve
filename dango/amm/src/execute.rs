use {
    crate::{perform_swap, PoolExt, PoolInit, CONFIG, NEXT_POOL_ID, POOLS},
    anyhow::{anyhow, ensure},
    dango_types::{
        amm::{
            ConcentratedPool, ExecuteMsg, InstantiateMsg, Pool, PoolId, PoolParams, XykPool,
            MINIMUM_LIQUIDITY, NAMESPACE, SUBNAMESPACE,
        },
        bank, taxman,
    },
    grug::{
        Coins, Denom, Inner, IsZero, Message, MutableCtx, Number, Part, Response, StdResult,
        Uint128, UniqueVec,
    },
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> StdResult<Response> {
    CONFIG.save(ctx.storage, &msg.config)?;

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn execute(ctx: MutableCtx, msg: ExecuteMsg) -> anyhow::Result<Response> {
    match msg {
        ExecuteMsg::CreatePool(params) => create_pool(ctx, params),
        ExecuteMsg::Swap {
            route,
            minimum_output,
        } => swap(ctx, route, minimum_output),
        ExecuteMsg::ProvideLiquidity {
            pool_id,
            minimum_output,
        } => provide_liquidity(ctx, pool_id, minimum_output),
        ExecuteMsg::WithdrawLiquidity { pool_id } => withdraw_liquidity(ctx, pool_id),
    }
}

fn create_pool(ctx: MutableCtx, params: PoolParams) -> anyhow::Result<Response> {
    let amm_cfg = CONFIG.load(ctx.storage)?;
    let mut liquidity = ctx.funds.clone();

    // Deduct the pool creation fee from the deposit.
    // The rest of the deposit goes into the pool as initial liquidity.
    liquidity
        .deduct(amm_cfg.pool_creation_fee.inner().clone())
        .map_err(|_| {
            anyhow!(
                "insufficient deposit to cover pool creation fee: {} < {}",
                ctx.funds,
                amm_cfg.pool_creation_fee,
            )
        })?;

    let (pool_id, _) = NEXT_POOL_ID.increment(ctx.storage)?;

    let (shares_to_mint, pool) = match params {
        PoolParams::Xyk(params) => {
            let xyk = XykPool::initialize(liquidity.try_into()?, params)?;
            (xyk.shares, Pool::Xyk(xyk))
        },
        PoolParams::Concentracted(params) => {
            let concentrated = ConcentratedPool::initialize(liquidity.try_into()?, params)?;
            (concentrated.shares, Pool::Concentrated(concentrated))
        },
    };

    // A minimum amount of liquidity tokens is to be withheld by the contract,
    // in order to prevent share price manipulation attack:
    // > https://docs.openzeppelin.com/contracts/4.x/erc4626#inflation-attack
    // Error if the shares to mint is less than the minimum liquidity.
    let shares_to_mint = shares_to_mint.checked_sub(MINIMUM_LIQUIDITY).map_err(|_| {
        anyhow!(
            "insufficient initial liquidity: {} < {}",
            shares_to_mint,
            MINIMUM_LIQUIDITY
        )
    })?;

    POOLS.save(ctx.storage, pool_id, &pool)?;

    let cfg = ctx.querier.query_config()?;
    let denom = denom_of(pool_id)?;

    // 1. Mint self the withheld liquidity tokens.
    // 2. Mint the creator the remaining liquidity tokens.
    // 3. Forward the pool creation fee to taxman.
    Ok(Response::new()
        .add_message(Message::execute(
            cfg.bank,
            &bank::ExecuteMsg::Mint {
                to: ctx.contract,
                denom: denom.clone(),
                amount: MINIMUM_LIQUIDITY,
            },
            Coins::new(),
        )?)
        .add_message(Message::execute(
            cfg.bank,
            &bank::ExecuteMsg::Mint {
                to: ctx.sender,
                denom,
                amount: shares_to_mint,
            },
            Coins::new(),
        )?)
        .add_message(Message::execute(
            cfg.taxman,
            &taxman::ExecuteMsg::Pay { payer: ctx.sender },
            amm_cfg.pool_creation_fee.into_inner(),
        )?))
}

fn swap(
    ctx: MutableCtx,
    route: UniqueVec<PoolId>,
    minimum_output: Option<Uint128>,
) -> anyhow::Result<Response> {
    let amm_cfg = CONFIG.load(ctx.storage)?;
    let input = ctx.funds.into_one_coin()?;
    let mut pools = route
        .inner()
        .iter()
        .map(|&pool_id| POOLS.load(ctx.storage, pool_id))
        .collect::<StdResult<Vec<_>>>()?;

    // Perform the swap in each pool.
    let outcome = perform_swap(&amm_cfg, input, pools.iter_mut())?;

    if let Some(minimum_output) = minimum_output {
        ensure!(
            outcome.output.amount >= minimum_output,
            "insufficient swap output: {} < {}",
            outcome.output.amount,
            minimum_output
        );
    }

    // Save the updated pool states.
    for (pool_id, pool) in route.inner().iter().zip(pools) {
        POOLS.save(ctx.storage, *pool_id, &pool)?;
    }

    // Transfer the post-fee output, if non-zero, to the trader.
    let output_msg = if outcome.output.is_non_zero() {
        Some(Message::transfer(ctx.sender, outcome.output)?)
    } else {
        None
    };

    // Transfer the protocol fee, if non-zero, to taxman.
    let fee_msg = if outcome.protocol_fee.is_non_zero() {
        let cfg = ctx.querier.query_config()?;

        Some(Message::execute(
            cfg.taxman,
            &taxman::ExecuteMsg::Pay { payer: ctx.sender },
            outcome.protocol_fee,
        )?)
    } else {
        None
    };

    Ok(Response::new()
        .may_add_message(output_msg)
        .may_add_message(fee_msg))
}

fn provide_liquidity(
    mut ctx: MutableCtx,
    pool_id: PoolId,
    minimum_output: Option<Uint128>,
) -> anyhow::Result<Response> {
    let mut pool = POOLS.load(ctx.storage, pool_id)?;

    let deposit = ctx.funds.take_pair(pool.denoms())?;

    // Sender must not send any other funds than what goes into the pool.
    ensure!(ctx.funds.is_empty(), "unexpected funds: {}", ctx.funds);

    let shares_to_mint = match &mut pool {
        Pool::Xyk(xyk) => xyk.provide_liquidity(deposit)?,
        Pool::Concentrated(concentrated) => concentrated.provide_liquidity(deposit)?,
    };

    POOLS.save(ctx.storage, pool_id, &pool)?;

    if let Some(minimum_output) = minimum_output {
        ensure!(
            shares_to_mint >= minimum_output,
            "insufficient liquidity provision output: {} < {}",
            shares_to_mint,
            minimum_output
        );
    }

    let cfg = ctx.querier.query_config()?;
    let denom = denom_of(pool_id)?;

    Ok(Response::new().add_message(Message::execute(
        cfg.bank,
        &bank::ExecuteMsg::Mint {
            to: ctx.sender,
            denom,
            amount: shares_to_mint,
        },
        Coins::new(),
    )?))
}

fn withdraw_liquidity(ctx: MutableCtx, pool_id: PoolId) -> anyhow::Result<Response> {
    let denom = denom_of(pool_id)?;
    let coin_to_burn = ctx.funds.into_one_coin()?;

    ensure!(
        coin_to_burn.denom == denom,
        "invalid liquidity token: expected {}, got {}",
        denom,
        coin_to_burn.denom
    );

    let mut pool = POOLS.load(ctx.storage, pool_id)?;
    let shares_to_burn = coin_to_burn.amount;

    let refunds = match &mut pool {
        Pool::Xyk(xyk) => xyk.withdraw_liquidity(shares_to_burn)?,
        Pool::Concentrated(concentrated) => concentrated.withdraw_liquidity(shares_to_burn)?,
    };

    POOLS.save(ctx.storage, pool_id, &pool)?;

    let cfg = ctx.querier.query_config()?;

    Ok(Response::new()
        .add_message(Message::execute(
            cfg.bank,
            &bank::ExecuteMsg::Burn {
                from: ctx.contract,
                denom,
                amount: coin_to_burn.amount,
            },
            Coins::new(),
        )?)
        .add_message(Message::transfer(ctx.sender, refunds)?))
}

/// Returns the LP token denom of the given pool.
#[inline]
fn denom_of(pool_id: PoolId) -> StdResult<Denom> {
    // A pool ID is necessarily a valid `Part`.
    let pool_id = Part::new_unchecked(pool_id.to_string());

    Denom::from_parts([NAMESPACE.clone(), SUBNAMESPACE.clone(), pool_id])
}
