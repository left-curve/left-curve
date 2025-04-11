use {
    dango_lending::{DEBTS, MARKETS},
    dango_oracle::OracleQuerier,
    dango_types::{
        DangoQuerier,
        account::margin::HealthResponse,
        dex::{Direction, QueryOrdersByUserRequest},
    },
    grug::{
        Addr, Coin, Coins, Inner, IsZero, MultiplyFraction, Number, NumberConst, QuerierExt,
        QuerierWrapper, StorageQuerier, Timestamp, Udec128,
    },
    std::cmp::min,
};

/// Queries the health of the margin account.
///
/// Arguments:
///
/// - `account`: The margin account to query.
/// - `discount_collateral`: If set, does not include the value of these
///   coins in the total collateral value. Used when liquidating the
///   account as the liquidator has sent additional funds to the account
///   that should not be included in the total collateral value.
pub fn query_health(
    querier: &QuerierWrapper,
    account: Addr,
    current_time: Timestamp,
    discount_collateral: Option<Coins>,
) -> anyhow::Result<HealthResponse> {
    let app_cfg = querier.query_dango_config()?;

    // ------------------------------- 1. Debts --------------------------------

    // Query all debts for the account.
    let scaled_debts = querier
        .may_query_wasm_path(app_cfg.addresses.lending, DEBTS.path(account))?
        .unwrap_or_default();

    // Calculate the total value of the debts.
    let mut debts = Coins::new();
    let mut total_debt_value = Udec128::ZERO;

    // Iterate over the scaled debt amounts and add the debt value to the total
    // debt value.
    for (denom, scaled_debt) in &scaled_debts {
        // Query the market for the denom.
        let market = querier
            .query_wasm_path(app_cfg.addresses.lending, &MARKETS.path(denom))?
            .update_indices(querier, current_time)?;

        // Calculate the real debt.
        let debt = market.calculate_debt(*scaled_debt)?;
        debts.insert(Coin::new(denom.clone(), debt)?)?;

        // Calculate the value of the debt.
        let price = querier.query_price(app_cfg.addresses.oracle, denom, None)?;
        let value = price.value_of_unit_amount(debt)?;

        total_debt_value.checked_add_assign(value)?;
    }

    // ---------------------------- 2. Collaterals -----------------------------

    // Calculate the total value of the account's collateral adjusted for the
    // collateral power.
    let mut total_collateral_value = Udec128::ZERO;
    let mut total_adjusted_collateral_value = Udec128::ZERO;
    let mut collaterals = Coins::new();

    // Iterate over the collateral powers and add the collateral value to the
    // total adjusted collateral value.
    for (denom, power) in &app_cfg.collateral_powers {
        let mut collateral_balance = querier.query_balance(account, denom.clone())?;

        if let Some(discount_collateral) = discount_collateral.as_ref() {
            collateral_balance.checked_sub_assign(discount_collateral.amount_of(denom))?;
        }

        // As an optimization, don't query the price if the collateral balance
        // is zero.
        if collateral_balance.is_zero() {
            continue;
        }

        let price = querier.query_price(app_cfg.addresses.oracle, denom, None)?;
        let value = price.value_of_unit_amount(collateral_balance)?;
        let adjusted_value = value.checked_mul(power.into_inner())?;

        collaterals.insert(Coin::new(denom.clone(), collateral_balance)?)?;
        total_collateral_value.checked_add_assign(value)?;
        total_adjusted_collateral_value.checked_add_assign(adjusted_value)?;
    }

    // ---------------------------- 3. Limit Orders ----------------------------

    // Add assets locked in limit orders to the total adjusted collateral value.
    //
    // For BUY orders, the user have transferred the quote asset to the DEX;
    // conversely, for SELL orders, the user have transferred the base asset.
    //
    // The collateral value of a limit order is evaluated as either that of the
    // input asset, or that of the output asset, whichever is smaller.
    let mut limit_order_collaterals = Coins::new();
    let mut limit_order_outputs = Coins::new();

    // Query the user's open limit orders.
    let orders = querier.query_wasm_smart(app_cfg.addresses.dex, QueryOrdersByUserRequest {
        user: account,
        start_after: None,
        limit: None,
    })?;

    // Iterate over the user's limit orders and add the order value to the
    // total collateral value.
    for (_, res) in orders {
        // Get asset locked in the order and the asset that would be returned
        // if the order was filled.
        let (offer, ask) = match res.direction {
            Direction::Bid => (
                Coin::new(
                    res.quote_denom.clone(),
                    res.remaining.checked_mul_dec_ceil(res.price)?,
                )?,
                Coin::new(res.base_denom.clone(), res.remaining)?,
            ),
            Direction::Ask => (
                Coin::new(res.base_denom.clone(), res.remaining)?,
                Coin::new(
                    res.quote_denom.clone(),
                    res.remaining.checked_mul_dec_floor(res.price)?,
                )?,
            ),
        };

        let offer_price = querier.query_price(app_cfg.addresses.oracle, &offer.denom, None)?;
        let offer_value = offer_price.value_of_unit_amount(offer.amount)?;
        let offer_collateral_power = app_cfg
            .collateral_powers
            .get(&offer.denom)
            .map(|x| x.into_inner())
            .unwrap_or(Udec128::ZERO);
        let offer_adjusted_value = offer_value.checked_mul(offer_collateral_power)?;

        let ask_price = querier.query_price(app_cfg.addresses.oracle, &ask.denom, None)?;
        let ask_value = ask_price.value_of_unit_amount(ask.amount)?;
        let ask_collateral_power = app_cfg
            .collateral_powers
            .get(&ask.denom)
            .map(|x| x.into_inner())
            .unwrap_or(Udec128::ZERO);
        let ask_adjusted_value = ask_value.checked_mul(ask_collateral_power)?;

        let min_value = min(offer_value, ask_value);
        let min_adjusted_value = min(offer_adjusted_value, ask_adjusted_value);

        total_collateral_value.checked_add_assign(min_value)?;
        total_adjusted_collateral_value.checked_add_assign(min_adjusted_value)?;
        limit_order_collaterals.insert(offer)?;
        limit_order_outputs.insert(ask)?;
    }

    // -------------------------- 4. Utilization rate --------------------------

    // Calculate the utilization rate.
    let utilization_rate = if total_debt_value.is_zero() {
        // The account has no debt. Utilization is zero in this case, regardless
        // of collateral value.
        Udec128::ZERO
    } else if total_adjusted_collateral_value.is_zero() {
        // The account has non-zero debt but zero collateral. This can happen if
        // the account is liquidated. We set utilization to maximum.
        Udec128::MAX
    } else {
        total_debt_value / total_adjusted_collateral_value
    };

    Ok(HealthResponse {
        utilization_rate,
        total_debt_value,
        total_collateral_value,
        total_adjusted_collateral_value,
        debts,
        collaterals,
        limit_order_collaterals,
        limit_order_outputs,
    })
}
