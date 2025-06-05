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
        perps::{
            ExecuteMsg, InstantiateMsg, PerpsMarketAccumulators, PerpsMarketParams,
            PerpsMarketState, PerpsPosition, PerpsVaultState, Pnl, same_side,
        },
    },
    grug::{
        Coins, Dec128, Denom, Int128, IsZero, Message, MutableCtx, Number, NumberConst, Order,
        QuerierExt, Response, Sign, Signed, StdError, StorageQuerier, Udec128, Uint128, Unsigned,
    },
    std::collections::{BTreeMap, HashMap},
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> anyhow::Result<Response> {
    // Initialize the perps vault state
    PERPS_VAULT.save(ctx.storage, &PerpsVaultState {
        denom: msg.perps_vault_denom,
        deposits: Uint128::ZERO,
        shares: Uint128::ZERO,
        realised_pnl: Default::default(),
    })?;

    // Store the perps market params and initialize perps market states.
    for (denom, perps_market_params) in msg.perps_market_params {
        PERPS_MARKET_PARAMS.save(ctx.storage, &denom, &perps_market_params)?;
        PERPS_MARKETS.save(ctx.storage, &denom, &PerpsMarketState {
            denom: denom.clone(),
            long_oi: Uint128::ZERO,
            short_oi: Uint128::ZERO,
            last_updated: ctx.block.timestamp,
            last_funding_rate: Dec128::ZERO,
            last_funding_index: Dec128::ZERO,
            accumulators: PerpsMarketAccumulators::new(),
            realised_pnl: Default::default(),
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
    // Load the vault state
    let vault_state = PERPS_VAULT.load(ctx.storage)?;

    // Ensure only the vault denom is sent
    let deposited = ctx.funds.as_one_coin_of_denom(&vault_state.denom)?;

    // Load the market states and params
    let markets = PERPS_MARKETS
        .range(ctx.storage, None, None, Order::Ascending)
        .map(|x| x.map(|(_, v)| v))
        .collect::<Result<Vec<_>, _>>()?;
    let params = PERPS_MARKET_PARAMS
        .range(ctx.storage, None, None, Order::Ascending)
        .collect::<Result<HashMap<Denom, PerpsMarketParams>, _>>()?;

    // Query the oracle for the price of each market denom
    let mut oracle_querier = OracleQuerier::new_remote(ctx.querier.query_oracle()?, ctx.querier);
    let oracle_prices = markets
        .iter()
        .flat_map(|m| {
            oracle_querier
                .query_price(&m.denom, None)
                .map(|p| Ok((m.denom.clone(), p.unit_price()?)))
        })
        .collect::<Result<HashMap<Denom, Udec128>, StdError>>()?;

    // Calculate the number of shares to mint
    let shares = core::token_to_shares(
        &markets,
        &oracle_prices,
        &params,
        &vault_state,
        *deposited.amount,
    )?;

    // Update the vault state
    PERPS_VAULT.save(ctx.storage, &PerpsVaultState {
        denom: vault_state.denom,
        deposits: vault_state.deposits.checked_add(*deposited.amount)?,
        shares: vault_state.shares.checked_add(shares)?,
        realised_pnl: vault_state.realised_pnl,
    })?;

    // Store the deposit
    PERPS_VAULT_DEPOSITS.save(ctx.storage, &ctx.sender, &shares)?;

    Ok(Response::new())
}

fn withdraw(ctx: MutableCtx, withdrawn_shares: Uint128) -> anyhow::Result<Response> {
    // Load the user's deposit
    let user_deposit = PERPS_VAULT_DEPOSITS
        .may_load(ctx.storage, &ctx.sender)?
        .unwrap_or(Uint128::ZERO);

    // Ensure no funds are sent
    ensure!(ctx.funds.is_empty(), "no funds should be sent");

    // Ensure withdrawn shares is not zero
    ensure!(!withdrawn_shares.is_zero(), "withdrawn shares is zero");

    // Ensure the user has enough shares
    ensure!(
        user_deposit >= withdrawn_shares,
        "user does not have enough shares"
    );

    // Load the vault state
    let vault_state = PERPS_VAULT.load(ctx.storage)?;

    // Load the markets states and params
    let markets = PERPS_MARKETS
        .range(ctx.storage, None, None, Order::Ascending)
        .map(|x| x.map(|(_, v)| v))
        .collect::<Result<Vec<_>, _>>()?;
    let params = PERPS_MARKET_PARAMS
        .range(ctx.storage, None, None, Order::Ascending)
        .collect::<Result<HashMap<Denom, PerpsMarketParams>, _>>()?;

    // Query the oracle for the price of each market denom
    let mut oracle_querier = OracleQuerier::new_remote(ctx.querier.query_oracle()?, ctx.querier);
    let oracle_prices = markets
        .iter()
        .flat_map(|m| {
            oracle_querier
                .query_price(&m.denom, None)
                .map(|p| Ok((m.denom.clone(), p.unit_price()?)))
        })
        .collect::<Result<HashMap<Denom, Udec128>, StdError>>()?;

    // Calculate the amount of tokens to send
    let withdrawn_amount = core::shares_to_token(
        &markets,
        &oracle_prices,
        &params,
        &vault_state,
        withdrawn_shares,
    )?;

    if withdrawn_amount.is_zero() {
        bail!("withdrawn amount is zero");
    }

    // Update the vault state
    PERPS_VAULT.save(ctx.storage, &PerpsVaultState {
        denom: vault_state.denom.clone(),
        deposits: vault_state.deposits.checked_sub(withdrawn_amount)?,
        shares: vault_state.shares.checked_sub(withdrawn_shares)?,
        realised_pnl: vault_state.realised_pnl,
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
    mut ctx: MutableCtx,
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

    // For each order, modify the position
    for (denom, amount) in orders {
        modify_position(&mut ctx, denom, amount)?;
    }

    // TODO: Emit events

    Ok(Response::new())
}

fn modify_position(ctx: &mut MutableCtx, denom: Denom, amount: Int128) -> anyhow::Result<()> {
    let params = PERPS_MARKET_PARAMS.load(ctx.storage, &denom)?;
    let market_state = PERPS_MARKETS.load(ctx.storage, &denom)?;
    let vault_state = PERPS_VAULT.load(ctx.storage)?;

    let skew = market_state.skew()?;

    // Query the oracle for the price
    let mut oracle_querier = OracleQuerier::new_remote(ctx.querier.query_oracle()?, ctx.querier);
    let price = oracle_querier.query_price(&denom, None)?;
    let oracle_unit_price = price.unit_price()?.checked_into_signed()?;
    let vault_denom_price = oracle_querier.query_price(&vault_state.denom, None)?;

    // Query current position
    let current_pos = PERPS_POSITIONS
        .may_load(ctx.storage, (&ctx.sender, &denom))?
        .unwrap_or_else(|| PerpsPosition {
            denom: denom.clone(),
            size: Int128::ZERO,
            entry_price: Udec128::ZERO,
            entry_execution_price: Dec128::ZERO,
            entry_skew: skew,
            realized_pnl: Pnl::default(),
            entry_funding_index: market_state.last_funding_index,
        });

    // Calculate the new position size
    let new_size = current_pos.size.checked_add(amount)?;

    // Ensure the position is not too small
    if new_size.is_non_zero() {
        ensure!(
            price.value_of_unit_amount(new_size.unsigned_abs())?
                >= params.min_position_size.checked_into_dec()?,
            "position size is too small"
        );
    }

    // If increasing the position, ensure trading is enabled
    let changing_side = !same_side(current_pos.size, new_size);
    let increasing_mag = new_size.unsigned_abs() > current_pos.size.unsigned_abs();
    let increasing = increasing_mag || (changing_side && !new_size.is_zero());
    if increasing {
        ensure!(
            params.trading_enabled,
            "trading is not enabled for this market. you can only decrease your position size"
        );
    }

    // Calculate fill price
    let fill_price =
        market_state.calculate_fill_price(amount, oracle_unit_price, params.skew_scale)?;

    // TODO: assert slippage based on fill price

    // Calculate position unrealized pnl and order fee
    let position_unrealized_pnl = current_pos.unrealized_pnl(
        Some(amount),
        fill_price,
        &vault_denom_price,
        &market_state,
        &params,
    )?;
    let fee_in_vault_denom = position_unrealized_pnl.fees.unsigned_abs();

    // Ensure the fee is sent
    let sent_fee = ctx.funds.as_one_coin_of_denom(&vault_state.denom)?;
    ensure!(
        sent_fee.amount == &fee_in_vault_denom,
        "incorrect fee amount sent. sent: {}, expected: {}",
        sent_fee.amount,
        fee_in_vault_denom
    );

    // Validate position size against OI limits
    if amount.is_positive() {
        ensure!(
            market_state.long_oi.checked_add(amount.unsigned_abs())? <= params.max_long_oi,
            "position size would exceed max long oi"
        );
    } else {
        ensure!(
            market_state.short_oi.checked_add(amount.unsigned_abs())? <= params.max_short_oi,
            "position size would exceed max short oi"
        );
    }

    // Update funding rate
    let new_market_state =
        market_state.update_funding(&params, ctx.block.timestamp, &price, &vault_denom_price)?;

    // Create the new position
    let new_pos = PerpsPosition {
        size: new_size,
        entry_price: price.unit_price()?,
        entry_execution_price: fill_price,
        entry_skew: skew, // skew BEFORE trade
        entry_funding_index: new_market_state.last_funding_index,
        realized_pnl: current_pos.realized_pnl.add(&position_unrealized_pnl)?,
        denom: denom.clone(),
    };

    // Update the market accumulators
    let new_market_state = new_market_state.update_accumulators(&current_pos, &new_pos)?;

    // Update the cash flow
    let realised_cash_flow = position_unrealized_pnl.checked_neg()?;
    let new_market_state = PerpsMarketState {
        realised_pnl: market_state.realised_pnl.add(&realised_cash_flow)?,
        ..new_market_state
    };

    // Update the open interest of the market
    let new_market_state = new_market_state.update_open_interest(&current_pos, &new_pos)?;

    // Save the new market state
    PERPS_MARKETS.save(ctx.storage, &denom, &new_market_state)?;

    // Update the vault state
    PERPS_VAULT.save(ctx.storage, &PerpsVaultState {
        realised_pnl: vault_state.realised_pnl.add(&realised_cash_flow)?,
        ..vault_state
    })?;

    // Store the new position
    if new_size.is_zero() {
        PERPS_POSITIONS.remove(ctx.storage, (&ctx.sender, &denom));
    } else {
        PERPS_POSITIONS.save(ctx.storage, (&ctx.sender, &denom), &new_pos)?;
    }

    // TODO: Ensure any negative trader PnL is sent to the vault and send any positive trader PnL to the user

    // TODO: Emit events
    Ok(())
}
