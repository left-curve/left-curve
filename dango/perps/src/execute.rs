use {
    crate::{
        PERPS_MARKET_PARAMS, PERPS_MARKETS, PERPS_POSITIONS, PERPS_VAULT, PERPS_VAULT_DEPOSITS,
        core,
    },
    anyhow::{anyhow, bail, ensure},
    dango_account_factory::ACCOUNTS,
    dango_oracle::OracleQuerier,
    dango_types::{
        DangoQuerier,
        perps::{
            ExecuteMsg, InstantiateMsg, PerpsMarketAccumulators, PerpsMarketParams,
            PerpsMarketState, PerpsPosition, PerpsVaultState,
        },
    },
    grug::{
        Coins, Dec128, Denom, Inner, Int128, IsZero, Message, MultiplyFraction, MutableCtx, Number,
        NumberConst, QuerierExt, Response, Sign, Signed, StorageQuerier, Udec128, Uint128,
        Unsigned,
    },
    std::collections::BTreeMap,
};

pub const NANOSECONDS_PER_DAY: u128 = 86_400_000_000_000;
pub const MAX_FUNDING_RATE: Dec128 = Dec128::new_percent(96);

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> anyhow::Result<Response> {
    // Initialize the perps vault state
    PERPS_VAULT.save(ctx.storage, &PerpsVaultState {
        denom: msg.perps_vault_denom,
        deposits: Uint128::ZERO,
        shares: Uint128::ZERO,
        realised_cash_flow: Default::default(),
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
        realised_cash_flow: vault_state.realised_cash_flow,
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
        realised_cash_flow: vault_state.realised_cash_flow,
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
    let position_value = price.value_of_unit_amount(amount.unsigned_abs())?;

    // Ensure minimum position value
    ensure!(
        position_value.into_int() >= params.min_position_size,
        "position value is below the minimum position value"
    );

    // TODO: Implement max position size ?

    Ok(Response::new())
}

fn modify_position(
    ctx: &mut MutableCtx,
    denom: Denom,
    amount: Int128,
    params: &PerpsMarketParams,
) -> anyhow::Result<Response> {
    let market_state = PERPS_MARKETS.load(ctx.storage, &denom)?;
    let vault_state = PERPS_VAULT.load(ctx.storage)?;

    let skew = market_state.skew()?;

    // Query the oracle for the price
    let mut oracle_querier = OracleQuerier::new_remote(ctx.querier.query_oracle()?, ctx.querier);
    let price = oracle_querier.query_price(&denom, None)?;
    let oracle_unit_price = price.unit_price()?.checked_into_signed()?;
    let position_value = price.value_of_unit_amount(amount.unsigned_abs())?;

    // Query current position
    let current_pos = PERPS_POSITIONS
        .may_load(ctx.storage, (&ctx.sender, &denom))?
        .unwrap_or_else(|| PerpsPosition {
            denom: denom.clone(),
            size: Int128::ZERO,
            entry_price: Udec128::ZERO,
            entry_execution_price: Udec128::ZERO,
            entry_skew: skew,
            realized_pnl: Int128::ZERO,
            entry_funding_index: market_state.last_funding_index,
        });

    // Calculate fill price
    let skew_scale = params.skew_scale.checked_into_signed()?;
    let pd_before = Dec128::checked_from_ratio(skew, skew_scale)?;
    let pd_after = Dec128::checked_from_ratio(skew.checked_add(amount)?, skew_scale)?;
    let price_before = oracle_unit_price.checked_add(oracle_unit_price.checked_mul(pd_before)?)?;
    let price_after = oracle_unit_price.checked_add(oracle_unit_price.checked_mul(pd_after)?)?;
    let fill_price = price_before
        .checked_add(price_after)?
        .checked_div(Dec128::new(2))?;

    // TODO: assert slippage based on fill price

    // Calculate order fee
    let notional_diff = amount.checked_mul_dec(fill_price)?;

    // Check if trade keeps skew on one side
    let new_skew = skew.checked_add(amount)?;
    let fee_usd = if same_side(skew, new_skew) {
        // This trade keeps the skew on the same side.
        let fee_rate = if same_side(notional_diff, skew) {
            params.taker_fee
        } else {
            params.maker_fee
        };

        fee_rate
            .into_inner()
            .checked_mul(notional_diff.unsigned_abs().checked_into_dec()?)?
    } else {
        // This trade flips the skew. Apply maker fee on the portion that
        // decreases the skew towards zero, and taker fee on the portion that
        // increases the skew away from zero.
        let taker_portion =
            Dec128::checked_from_ratio(amount.checked_add(skew)?, amount)?.unsigned_abs();
        let maker_portion = Udec128::ONE.checked_sub(taker_portion)?;
        let taker_fee = taker_portion
            .checked_mul(params.taker_fee.into_inner())?
            .checked_mul(notional_diff.unsigned_abs().checked_into_dec()?)?;
        let maker_fee = maker_portion
            .checked_mul(params.maker_fee.into_inner())?
            .checked_mul(notional_diff.unsigned_abs().checked_into_dec()?)?;

        taker_fee.checked_add(maker_fee)?
    };

    // Convert fee to vault denom
    let vault_denom_price = oracle_querier.query_price(&vault_state.denom, None)?;
    let fee_in_vault_denom = vault_denom_price.unit_amount_from_value(fee_usd)?;

    // Ensure the fee is sent
    let sent_fee = ctx.funds.as_one_coin_of_denom(&vault_state.denom)?;
    ensure!(sent_fee.amount == &fee_in_vault_denom, "fee is not sent");

    // Validate position size against OI limits
    if amount.is_positive() {
        ensure!(
            market_state.long_oi.checked_add(amount.unsigned_abs())? <= params.max_long_oi,
            "long position size is too large"
        );
    } else {
        ensure!(
            market_state.short_oi.checked_add(amount.unsigned_abs())? <= params.max_short_oi,
            "short position size is too large"
        );
    }

    // Update funding rate
    let time_elapsed_days = Udec128::checked_from_ratio(
        ctx.block
            .timestamp
            .into_nanos()
            .checked_sub(market_state.last_updated.into_nanos())
            .ok_or_else(|| anyhow!("time elapsed is negative"))?,
        NANOSECONDS_PER_DAY,
    )?
    .checked_into_signed()?;
    let proportional_skew = market_state.proportional_skew(params.skew_scale)?;
    let current_funding_velocity =
        proportional_skew.checked_mul(params.max_funding_velocity.checked_into_signed()?)?;
    let funding_rate = market_state
        .last_funding_rate
        .checked_add(current_funding_velocity.checked_mul(time_elapsed_days)?)?;
    let funding_rate = funding_rate.clamp(-MAX_FUNDING_RATE, MAX_FUNDING_RATE);

    // Update current funding index
    let average_funding_rate = market_state
        .last_funding_rate
        .checked_add(funding_rate)?
        .checked_div(Dec128::ONE + Dec128::ONE)?;
    let market_denom_price_in_vault_denom = price
        .unit_price()?
        .checked_div(vault_denom_price.unit_price()?)?;
    let unrecorded_funding = average_funding_rate
        .checked_mul(time_elapsed_days)?
        .checked_mul(market_denom_price_in_vault_denom.checked_into_signed()?)?;
    let funding_index = market_state
        .last_funding_index
        .checked_sub(unrecorded_funding)?;

    // Create the new position
    let new_size = current_pos.size.checked_add(amount)?;
    let mut new_pos = PerpsPosition {
        size: new_size,
        entry_price: price.unit_price()?,
        entry_execution_price: fill_price.checked_into_unsigned()?,
        entry_skew: skew, // skew BEFORE trade
        entry_funding_index: funding_index,
        realized_pnl: current_pos.realized_pnl, // Will be updated later
        denom: denom.clone(),
    };

    // Update the market accumulators
    let mut accumulators = market_state.accumulators;
    if current_pos.size.is_non_zero() {
        accumulators.decrease(&current_pos)?;
    }
    accumulators.increase(&new_pos)?;

    // Update the market state
    let mut long_oi = market_state.long_oi;
    let mut short_oi = market_state.short_oi;
    if current_pos.size.is_positive() {
        long_oi = long_oi.checked_sub(current_pos.size.unsigned_abs())?;
    } else {
        short_oi = short_oi.checked_sub(current_pos.size.unsigned_abs())?;
    }
    if new_pos.size.is_positive() {
        long_oi = long_oi.checked_add(new_pos.size.unsigned_abs())?;
    } else {
        short_oi = short_oi.checked_add(new_pos.size.unsigned_abs())?;
    }
    let new_market_state = PerpsMarketState {
        long_oi,
        short_oi,
        last_updated: ctx.block.timestamp,
        last_funding_rate: funding_rate,
        last_funding_index: funding_index,
        accumulators,
        denom: denom.clone(),
    };
    PERPS_MARKETS.save(ctx.storage, &denom, &new_market_state)?;

    // Update the cash flow
    let mut realised_cash_flow = vault_state.realised_cash_flow;
    if amount.unsigned_abs() > current_pos.size.unsigned_abs() {
        // position notional ↑
        realised_cash_flow
            .opening_fee
            .checked_add_assign(fee_in_vault_denom.checked_into_signed()?)?;
    } else {
        realised_cash_flow
            .closing_fee
            .checked_add_assign(fee_in_vault_denom.checked_into_signed()?)?;
    }

    let q_closed = if same_side(amount, current_pos.size) {
        Int128::ZERO // pure increase or pure flip across zero
    } else {
        // Reduce or fully close in the opposite direction
        // clamp so we don't overshoot
        if amount.unsigned_abs() >= current_pos.size.unsigned_abs() {
            current_pos.size
        } else {
            amount
        }
    };
    if !q_closed.is_zero() {
        let realised_price_pnl = q_closed.checked_mul_dec(
            fill_price
                .checked_sub(current_pos.entry_execution_price.checked_into_signed()?)?
                .checked_div(vault_denom_price.unit_price()?.checked_into_signed()?)?,
        )?;
        let realised_funding_pnl =
            q_closed.checked_mul_dec(funding_index - current_pos.entry_funding_index)?;
        realised_cash_flow
            .price_pnl
            .checked_add_assign(realised_price_pnl)?;
        realised_cash_flow
            .accrued_funding
            .checked_add_assign(realised_funding_pnl)?;

        // realised ΔPnL for the trader
        let pnl_delta = realised_price_pnl
            .checked_add(realised_funding_pnl)?
            .checked_sub(fee_in_vault_denom.checked_into_signed()?)?;
        new_pos.realized_pnl = new_pos.realized_pnl.checked_add(pnl_delta)?;
    }

    // Update the vault state
    PERPS_VAULT.save(ctx.storage, &PerpsVaultState {
        realised_cash_flow,
        ..vault_state
    })?;

    // Store the new position
    if new_size.is_zero() {
        PERPS_POSITIONS.remove(ctx.storage, (&ctx.sender, &denom));
    } else {
        PERPS_POSITIONS.save(ctx.storage, (&ctx.sender, &denom), &new_pos)?;
    }

    Ok(Response::new())
}

fn same_side(a: Int128, b: Int128) -> bool {
    (a.is_positive() == b.is_positive()) || a.is_zero() || b.is_zero()
}
