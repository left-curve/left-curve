use {
    crate::{PERPS_MARKET_PARAMS, PERPS_VAULT_DEPOSITS, state::PERPS_VAULT},
    dango_types::perps::{PerpsMarketParams, PerpsVaultState, QueryMsg},
    grug::{
        Addr, Bound, DEFAULT_PAGE_LIMIT, Denom, ImmutableCtx, Json, JsonSerExt, Order, Uint128,
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
