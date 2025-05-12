use {
    crate::{
        PERPS_MARKET_PARAMS, PERPS_MARKETS, PERPS_POSITIONS, PERPS_VAULT, PERPS_VAULT_DEPOSITS,
        core,
    },
    anyhow::{bail, ensure},
    dango_account_factory::ACCOUNTS,
    dango_oracle::OracleQuerier,
    dango_types::{
        DangoQuerier,
        perps::{ExecuteMsg, InstantiateMsg, PerpsMarketParams, PerpsMarketState, PerpsVaultState},
    },
    grug::{
        Coins, Denom, Int128, IsZero, Message, MutableCtx, Number, NumberConst, QuerierExt,
        Response, Sign, Signed, StorageQuerier, Uint128,
    },
    std::collections::BTreeMap,
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> anyhow::Result<Response> {
    // Initialize the perps vault state
    PERPS_VAULT.save(ctx.storage, &PerpsVaultState {
        denom: msg.perps_vault_denom,
        deposits: Uint128::ZERO,
        shares: Uint128::ZERO,
    })?;

    // Store the perps market params and initialize perps market states.
    for (denom, perps_market_params) in msg.perps_market_params {
        PERPS_MARKET_PARAMS.save(ctx.storage, &denom, &perps_market_params)?;
        PERPS_MARKETS.save(ctx.storage, &denom, &PerpsMarketState {
            denom: denom.clone(),
            long_oi: Uint128::ZERO,
            short_oi: Uint128::ZERO,
            last_updated: ctx.block.timestamp,
        })?;
    }

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn execute(ctx: MutableCtx, msg: ExecuteMsg) -> anyhow::Result<Response> {
    match msg {
        ExecuteMsg::Deposit {} => deposit(ctx),
        ExecuteMsg::Withdraw { shares } => withdraw(ctx, shares),
        ExecuteMsg::BatchUpdateOrders { orders } => batch_update_orders(ctx, orders),
        ExecuteMsg::UpdatePerpsMarketParams { params } => update_perps_market_params(ctx, params),
    }
}

fn update_perps_market_params(
    ctx: MutableCtx,
    params: BTreeMap<Denom, PerpsMarketParams>,
) -> anyhow::Result<Response> {
    // Ensure the sender is the contract admin
    ensure!(
        ctx.sender == ctx.querier.query_owner()?,
        "you don't have the right, O you don't have the right"
    );

    for (denom, params) in params {
        PERPS_MARKET_PARAMS.save(ctx.storage, &denom, &params)?;
    }

    Ok(Response::new())
}

fn deposit(ctx: MutableCtx) -> anyhow::Result<Response> {
    let vault_state = PERPS_VAULT.load(ctx.storage)?;

    // Ensure only the vault denom is sent
    let deposited = ctx.funds.as_one_coin_of_denom(&vault_state.denom)?;

    let shares = core::token_to_shares(&vault_state, *deposited.amount)?;

    // Update the vault state
    PERPS_VAULT.save(ctx.storage, &PerpsVaultState {
        denom: vault_state.denom,
        deposits: vault_state.deposits.checked_add(*deposited.amount)?,
        shares: vault_state.shares.checked_add(shares)?,
    })?;

    // Store the deposit
    PERPS_VAULT_DEPOSITS.save(ctx.storage, &ctx.sender, &shares)?;

    Ok(Response::new())
}

fn withdraw(ctx: MutableCtx, withdrawn_shares: Uint128) -> anyhow::Result<Response> {
    // Ensure no funds are sent
    ensure!(ctx.funds.is_empty(), "no funds should be sent");

    let vault_state = PERPS_VAULT.load(ctx.storage)?;

    // Load the user's deposit
    let user_deposit = PERPS_VAULT_DEPOSITS
        .may_load(ctx.storage, &ctx.sender)?
        .unwrap_or(Uint128::ZERO);

    // Ensure the user has enough shares
    ensure!(
        user_deposit >= withdrawn_shares,
        "user does not have enough shares"
    );

    // Calculate the amount of tokens to send
    let withdrawn_amount = core::shares_to_token(&vault_state, withdrawn_shares)?;

    if withdrawn_amount.is_zero() {
        bail!("withdrawn amount is zero");
    }

    // Update the vault state
    PERPS_VAULT.save(ctx.storage, &PerpsVaultState {
        denom: vault_state.denom.clone(),
        deposits: vault_state.deposits.checked_sub(withdrawn_amount)?,
        shares: vault_state.shares.checked_sub(withdrawn_shares)?,
    })?;

    // Update the user's deposit
    PERPS_VAULT_DEPOSITS.save(
        ctx.storage,
        &ctx.sender,
        &user_deposit.checked_sub(withdrawn_shares)?,
    )?;

    // Send the tokens to the user
    let send_msg = Message::transfer(ctx.sender, Coins::one(vault_state.denom, withdrawn_amount)?)?;

    Ok(Response::new().add_message(send_msg))
}

fn batch_update_orders(
    ctx: MutableCtx,
    orders: BTreeMap<Denom, Int128>,
) -> anyhow::Result<Response> {
    let account_factory = ctx.querier.query_account_factory()?;

    // Ensure sender is a margin account.
    // An an optimization, use raw instead of smart query.
    ensure!(
        ctx.querier
            .query_wasm_path(account_factory, &ACCOUNTS.path(ctx.sender))?
            .params
            .is_margin(),
        "only margin accounts can update orders"
    );

    for (denom, amount) in orders {
        let market_params = PERPS_MARKET_PARAMS.load(ctx.storage, &denom)?;

        let current_pos = PERPS_POSITIONS.may_load(ctx.storage, (&ctx.sender, &denom))?;
        match current_pos {
            Some(pos) => {
                todo!()
            },
            None => {
                create_position(&ctx, denom, amount, &market_params)?;
            },
        }
    }

    Ok(Response::new())
}

fn create_position(
    ctx: &MutableCtx,
    denom: Denom,
    amount: Int128,
    params: &PerpsMarketParams,
) -> anyhow::Result<Response> {
    let market_state = PERPS_MARKETS.load(ctx.storage, &denom)?;

    // Ensure trading is enabled
    ensure!(
        params.trading_enabled,
        "trading is not enabled for this market"
    );

    // Query the oracle for the price
    let mut oracle_querier = OracleQuerier::new_remote(ctx.querier.query_oracle()?, ctx.querier);
    let price = oracle_querier.query_price(&denom, None)?;
    let position_value =
        price.value_of_unit_amount(amount.checked_abs()?.checked_into_unsigned()?)?;

    Ok(Response::new())
}
