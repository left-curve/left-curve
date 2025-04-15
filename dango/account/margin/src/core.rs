use {
    dango_lending::{DEBTS, MARKETS},
    dango_oracle::OracleQuerier,
    dango_types::{
        DangoQuerier,
        account::margin::{CollateralPower, HealthResponse},
        dex::{Direction, OrdersByUserResponse, QueryOrdersByUserRequest},
        lending::Market,
        oracle::PrecisionedPrice,
    },
    grug::{
        Addr, Coin, Coins, Denom, Inner, IsZero, MultiplyFraction, Number, NumberConst, QuerierExt,
        QuerierWrapper, StorageQuerier, Timestamp, Udec128, Udec256, Uint128,
    },
    std::{
        cmp::min,
        collections::{BTreeMap, HashSet},
    },
};

/// Queries the health of the margin account.
///
/// ## Inputs
///
/// - `account`: The margin account to query.
///
/// - `discount_collateral`: If set, does not include the value of these
///   coins in the total collateral value. Used when liquidating the
///   account as the liquidator has sent additional funds to the account
///   that should not be included in the total collateral value.
///
/// ## Outputs
///
/// - a `HealthResponse` struct containing the health of the margin account.
pub fn query_health(
    querier: &QuerierWrapper,
    account: Addr,
    current_time: Timestamp,
    discount_collateral: Option<Coins>,
) -> anyhow::Result<HealthResponse> {
    // ------------------------ 1. Query necessary data ------------------------

    let app_cfg = querier.query_dango_config()?;
    let collateral_powers = app_cfg.collateral_powers;

    // Query all debts for the account.
    let scaled_debts = querier
        .may_query_wasm_path(app_cfg.addresses.lending, DEBTS.path(account))?
        .unwrap_or_default();

    // Query all markets.
    let markets = scaled_debts
        .keys()
        .map(|denom| {
            let market = querier
                .query_wasm_path(app_cfg.addresses.lending, &MARKETS.path(denom))?
                .update_indices(querier, current_time)?;

            Ok((denom.clone(), market))
        })
        .collect::<anyhow::Result<BTreeMap<_, _>>>()?;

    // Query collateral balances.
    let collateral_balances = collateral_powers
        .keys()
        .map(|denom| {
            let balance = querier.query_balance(account, denom.clone())?;
            Ok((denom.clone(), balance))
        })
        .collect::<anyhow::Result<BTreeMap<_, _>>>()?;

    // Query all limit orders for the account.
    let limit_orders =
        querier.query_wasm_smart(app_cfg.addresses.dex, QueryOrdersByUserRequest {
            user: account,
            start_after: None,
            limit: None,
        })?;

    // Query all prices.
    let denoms = markets
        .clone()
        .into_keys()
        .chain(collateral_powers.keys().cloned())
        .chain(limit_orders.values().map(|res| res.base_denom.clone()))
        .chain(limit_orders.values().map(|res| res.quote_denom.clone()))
        .collect::<HashSet<_>>();

    let prices = denoms
        .iter()
        .map(|denom| {
            let price = querier.query_price(app_cfg.addresses.oracle, denom, None)?;
            Ok((denom.clone(), price))
        })
        .collect::<anyhow::Result<BTreeMap<_, _>>>()?;

    // --------------------------- 2. Compute health ---------------------------

    compute_health(
        discount_collateral,
        scaled_debts,
        markets,
        prices,
        collateral_powers,
        collateral_balances,
        limit_orders,
    )
}

/// Computes the health of the margin account.
///
/// ## Inputs
///
/// - `discount_collateral`: If set, does not include the value of these
///   coins in the total collateral value. Used when liquidating the
///   account as the liquidator has sent additional funds to the account
///   that should not be included in the total collateral value.
///
/// - `scaled_debts`: The debts scaled of the margin account.
///
/// - `markets`: The markets for the debt denoms of the margin account.
///
/// - `prices`: A map of all relevant denoms to their prices.
///
/// - `collateral_powers`: All registered collateral powers.
///
/// - `collateral_balances`: The margin account's balances of collateral tokens.
///
/// - `limit_orders`: All limit orders for the margin account.
///
/// ## Outputs
///
/// - a `HealthResponse` struct containing the health of the margin account.
pub fn compute_health(
    discount_collateral: Option<Coins>,
    scaled_debts: BTreeMap<Denom, Udec256>,
    markets: BTreeMap<Denom, Market>,
    prices: BTreeMap<Denom, PrecisionedPrice>,
    collateral_powers: BTreeMap<Denom, CollateralPower>,
    collateral_balances: BTreeMap<Denom, Uint128>,
    limit_orders: BTreeMap<u64, OrdersByUserResponse>,
) -> anyhow::Result<HealthResponse> {
    // ------------------------------- 1. Debts --------------------------------

    let mut debts = Coins::new();
    let mut total_debt_value = Udec128::ZERO;

    for (denom, scaled_debt) in &scaled_debts {
        // Get the market for the denom.
        let market = markets
            .get(denom)
            .ok_or(anyhow::anyhow!("market for denom {} not found", denom))?;

        // Calculate the real debt.
        let debt = market.calculate_debt(*scaled_debt)?;
        debts.insert(Coin::new(denom.clone(), debt)?)?;

        // Calculate the value of the debt.
        let price = prices
            .get(denom)
            .ok_or(anyhow::anyhow!("price for denom {} not found", denom))?;
        let value = price.value_of_unit_amount(debt)?;

        total_debt_value.checked_add_assign(value)?;
    }

    // ---------------------------- 2. Collaterals -----------------------------

    let mut total_collateral_value = Udec128::ZERO;
    let mut total_adjusted_collateral_value = Udec128::ZERO;
    let mut collaterals = Coins::new();

    for (denom, power) in &collateral_powers {
        let mut collateral_balance = *collateral_balances.get(denom).ok_or(anyhow::anyhow!(
            "collateral balance for denom {} not found",
            denom
        ))?;

        if let Some(discount_collateral) = discount_collateral.as_ref() {
            collateral_balance.checked_sub_assign(discount_collateral.amount_of(denom))?;
        }

        // As an optimization, don't query the price if the collateral balance
        // is zero.
        if collateral_balance.is_zero() {
            continue;
        }

        let price = prices
            .get(denom)
            .ok_or(anyhow::anyhow!("price for denom {} not found", denom))?;
        let value = price.value_of_unit_amount(collateral_balance)?;
        let adjusted_value = value.checked_mul(power.clone().into_inner())?;

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

    // Iterate over the user's limit orders and add the order value to the
    // total collateral value.
    for res in limit_orders.values() {
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

        let offer_price = prices
            .get(&offer.denom)
            .ok_or(anyhow::anyhow!("price for denom {} not found", offer.denom))?;
        let offer_value = offer_price.value_of_unit_amount(offer.amount)?;
        let offer_collateral_power = collateral_powers.get(&offer.denom).ok_or(anyhow::anyhow!(
            "collateral power for denom {} not found",
            offer.denom
        ))?;
        let offer_adjusted_value =
            offer_value.checked_mul(offer_collateral_power.clone().into_inner())?;

        let ask_price = prices
            .get(&ask.denom)
            .ok_or(anyhow::anyhow!("price for denom {} not found", ask.denom))?;
        let ask_value = ask_price.value_of_unit_amount(ask.amount)?;
        let ask_collateral_power = collateral_powers.get(&ask.denom).ok_or(anyhow::anyhow!(
            "collateral power for denom {} not found",
            ask.denom
        ))?;
        let ask_adjusted_value =
            ask_value.checked_mul(ask_collateral_power.clone().into_inner())?;

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
