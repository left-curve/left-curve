use {
    crate::{
        PERPS_MARKET_PARAMS, PERPS_MARKETS, PERPS_POSITIONS, PERPS_VAULT_DEPOSITS,
        state::PERPS_VAULT,
    },
    dango_oracle::OracleQuerier,
    dango_types::{
        DangoQuerier,
        perps::{
            PerpsMarketParams, PerpsMarketState, PerpsPosition, PerpsPositionResponse,
            PerpsVaultState, QueryMsg,
        },
    },
    grug::{
        Addr, Bound, Cache, DEFAULT_PAGE_LIMIT, Denom, ImmutableCtx, Json, JsonSerExt, Order, Sign,
        StdError, Uint128, Unsigned,
    },
    std::collections::BTreeMap,
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> anyhow::Result<Json> {
    match msg {
        QueryMsg::PerpsVaultState {} => {
            let res = query_perps_vault_state(ctx)?;
            res.to_json_value()
        },
        QueryMsg::PerpsMarketParamsForDenom { denom } => {
            let res = query_perps_market_params_for_denom(ctx, denom)?;
            res.to_json_value()
        },
        QueryMsg::PerpsMarketParams { limit, start_after } => {
            let res = query_perps_market_params(ctx, limit, start_after)?;
            res.to_json_value()
        },
        QueryMsg::VaultSharesForUser { address } => {
            let res = query_vault_shares_for_user(ctx, address)?;
            res.to_json_value()
        },
        QueryMsg::VaultShares { limit, start_after } => {
            let res = query_vault_shares(ctx, limit, start_after)?;
            res.to_json_value()
        },
        QueryMsg::PerpsMarketStateForDenom { denom } => {
            let res = query_perps_market_state_for_denom(ctx, denom)?;
            res.to_json_value()
        },
        QueryMsg::PerpsMarketStates { limit, start_after } => {
            let res = query_perps_market_states(ctx, limit, start_after)?;
            res.to_json_value()
        },
        QueryMsg::PerpsPositionsForUser { address } => {
            let res = query_perps_positions_for_user(ctx, address)?;
            res.to_json_value()
        },
        QueryMsg::PerpsPositions { limit, start_after } => {
            let res = query_perps_positions(ctx, limit, start_after)?;
            res.to_json_value()
        },
    }
    .map_err(Into::into)
}

fn query_perps_vault_state(ctx: ImmutableCtx) -> anyhow::Result<PerpsVaultState> {
    let perps_vault_state = PERPS_VAULT.load(ctx.storage)?;
    Ok(perps_vault_state)
}

fn query_perps_market_params_for_denom(
    ctx: ImmutableCtx,
    denom: Denom,
) -> anyhow::Result<PerpsMarketParams> {
    let perps_market_params = PERPS_MARKET_PARAMS.load(ctx.storage, &denom)?;
    Ok(perps_market_params)
}

fn query_perps_market_params(
    ctx: ImmutableCtx,
    limit: Option<u32>,
    start_after: Option<Denom>,
) -> anyhow::Result<BTreeMap<Denom, PerpsMarketParams>> {
    let start = start_after.as_ref().map(Bound::Exclusive);
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT);

    PERPS_MARKET_PARAMS
        .range(ctx.storage, start, None, Order::Ascending)
        .take(limit as usize)
        .map(|res| {
            let (denom, perps_market_params) = res?;
            Ok((denom, perps_market_params))
        })
        .collect()
}

fn query_vault_shares_for_user(ctx: ImmutableCtx, address: Addr) -> anyhow::Result<Uint128> {
    let vault_shares = PERPS_VAULT_DEPOSITS.load(ctx.storage, &address)?;
    Ok(vault_shares)
}

fn query_vault_shares(
    ctx: ImmutableCtx,
    limit: Option<u32>,
    start_after: Option<Addr>,
) -> anyhow::Result<BTreeMap<Addr, Uint128>> {
    let start = start_after.as_ref().map(Bound::Exclusive);
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT);

    PERPS_VAULT_DEPOSITS
        .range(ctx.storage, start, None, Order::Ascending)
        .take(limit as usize)
        .map(|res| {
            let (address, vault_shares) = res?;
            Ok((address, vault_shares))
        })
        .collect()
}

fn query_perps_market_state_for_denom(
    ctx: ImmutableCtx,
    denom: Denom,
) -> anyhow::Result<PerpsMarketState> {
    let perps_market_state = PERPS_MARKETS.load(ctx.storage, &denom)?;
    Ok(perps_market_state)
}

fn query_perps_market_states(
    ctx: ImmutableCtx,
    limit: Option<u32>,
    start_after: Option<Denom>,
) -> anyhow::Result<BTreeMap<Denom, PerpsMarketState>> {
    let start = start_after.as_ref().map(Bound::Exclusive);
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT);

    PERPS_MARKETS
        .range(ctx.storage, start, None, Order::Ascending)
        .take(limit as usize)
        .map(|res| {
            let (denom, perps_market_state) = res?;
            Ok((denom, perps_market_state))
        })
        .collect()
}

fn query_perps_positions_for_user(
    ctx: ImmutableCtx,
    address: Addr,
) -> anyhow::Result<BTreeMap<Denom, PerpsPositionResponse>> {
    // Get the positions
    let perps_positions: BTreeMap<Denom, PerpsPosition> = PERPS_POSITIONS
        .prefix(&address)
        .range(ctx.storage, None, None, Order::Ascending)
        .collect::<Result<_, _>>()?;

    // Create oracle querier and fetch vault denom price
    let app_cfg = ctx.querier.query_dango_config()?;
    let mut oracle_querier = OracleQuerier::new_remote(app_cfg.addresses.oracle, ctx.querier);
    let vault_denom = PERPS_VAULT.load(ctx.storage)?.denom;
    let vault_denom_price = oracle_querier.query_price(&vault_denom, None)?;

    // Calculate unrealized pnl for each position and create the responses
    let mut responses = BTreeMap::new();
    for (denom, perps_position) in perps_positions {
        let market_state = PERPS_MARKETS.load(ctx.storage, &denom)?;
        let market_params = PERPS_MARKET_PARAMS.load(ctx.storage, &denom)?;
        let oracle_price = oracle_querier.query_price(&denom, None)?;
        let fill_price = market_state.calculate_fill_price(
            perps_position.size.checked_neg()?,
            oracle_price.unit_price()?.checked_into_signed()?,
            market_params.skew_scale,
        )?;
        let unrealized_pnl = perps_position.unrealized_pnl(
            None,
            fill_price,
            &vault_denom_price,
            &market_state,
            &market_params,
        )?;
        responses.insert(denom, PerpsPositionResponse {
            unrealized_pnl,
            denom: perps_position.denom,
            entry_execution_price: perps_position.entry_execution_price,
            entry_price: perps_position.entry_price,
            entry_skew: perps_position.entry_skew,
            entry_funding_index: perps_position.entry_funding_index,
            realized_pnl: perps_position.realized_pnl,
            size: perps_position.size,
        });
    }

    Ok(responses)
}

fn query_perps_positions(
    ctx: ImmutableCtx,
    limit: Option<u32>,
    start_after: Option<(Addr, Denom)>,
) -> anyhow::Result<BTreeMap<Addr, BTreeMap<Denom, PerpsPositionResponse>>> {
    let start = start_after
        .as_ref()
        .map(|(address, denom)| Bound::Exclusive((address, denom)));
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT);

    // Get the positions
    let positions: BTreeMap<(Addr, Denom), PerpsPosition> = PERPS_POSITIONS
        .range(ctx.storage, start, None, Order::Ascending)
        .take(limit as usize)
        .map(|res| {
            let ((address, denom), perps_positions) = res?;
            Ok::<_, StdError>(((address, denom), perps_positions))
        })
        .collect::<Result<_, _>>()?;

    // Create oracle querier and fetch vault denom price
    let app_cfg = ctx.querier.query_dango_config()?;
    let mut oracle_querier = OracleQuerier::new_remote(app_cfg.addresses.oracle, ctx.querier);
    let vault_denom = PERPS_VAULT.load(ctx.storage)?.denom;
    let vault_denom_price = oracle_querier.query_price(&vault_denom, None)?;

    // Create caches for market states and params
    let mut market_states =
        Cache::<Denom, PerpsMarketState>::new(|denom, _| PERPS_MARKETS.load(ctx.storage, denom));
    let mut market_params = Cache::<Denom, PerpsMarketParams>::new(|denom, _| {
        PERPS_MARKET_PARAMS.load(ctx.storage, denom)
    });

    // Calculate unrealized pnl for each position and create the responses
    let mut result = BTreeMap::new();
    for ((address, denom), perps_position) in positions {
        let market_state = market_states.get_or_fetch(&denom, None)?;
        let market_params = market_params.get_or_fetch(&denom, None)?;
        let oracle_price = oracle_querier.query_price(&denom, None)?;
        let fill_price = market_state.calculate_fill_price(
            perps_position.size,
            oracle_price.unit_price()?.checked_into_signed()?,
            market_params.skew_scale,
        )?;
        let unrealized_pnl = perps_position.unrealized_pnl(
            None,
            fill_price,
            &vault_denom_price,
            &market_state,
            &market_params,
        )?;

        let response = PerpsPositionResponse {
            unrealized_pnl,
            denom: perps_position.denom,
            entry_execution_price: perps_position.entry_execution_price,
            entry_price: perps_position.entry_price,
            entry_skew: perps_position.entry_skew,
            entry_funding_index: perps_position.entry_funding_index,
            realized_pnl: perps_position.realized_pnl,
            size: perps_position.size,
        };

        result
            .entry(address)
            .or_insert_with(BTreeMap::new)
            .insert(denom, response);
    }
    Ok(result)
}
