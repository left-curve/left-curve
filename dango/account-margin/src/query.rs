use {
    anyhow::bail,
    dango_auth::NEXT_SEQUENCE,
    dango_lending::DEBTS,
    dango_types::{
        account::margin::{HealthResponse, QueryMsg},
        config::AppConfig,
        lending::CollateralPower,
    },
    grug::{
        Addr, BorshDeExt, Coins, Denom, ImmutableCtx, Inner, IsZero, Json, JsonSerExt, NumberConst,
        QuerierWrapper, StdResult, Storage, Udec128,
    },
    std::collections::BTreeMap,
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> anyhow::Result<Json> {
    match msg {
        QueryMsg::Sequence {} => query_sequence(ctx.storage)?.to_json_value(),
        QueryMsg::Health {} => query_health(ctx)?.to_json_value(),
    }
    .map_err(Into::into)
}

fn query_sequence(storage: &dyn Storage) -> StdResult<u32> {
    NEXT_SEQUENCE.current(storage)
}

/// Calculates the health of a margin account.
pub fn calculate_account_health(
    querier: &QuerierWrapper,
    margin_account: Addr,
    debts: Coins,
    collateral_powers: BTreeMap<Denom, CollateralPower>,
) -> anyhow::Result<HealthResponse> {
    // Calculate the total value of the debts.
    let mut total_debt_value = Udec128::ZERO;
    for coin in debts {
        let price = dango_oracle::raw_query_price(querier, &coin.denom)?;
        total_debt_value += price.value_of_unit_amount(coin.amount);
    }

    // Calculate the total value of the account's collateral adjusted for the collateral power.
    let mut total_adjusted_collateral_value = Udec128::ZERO;
    for (denom, power) in collateral_powers {
        let collateral_balance = querier.query_balance(margin_account, denom.clone())?;

        // As an optimization, don't query the price if the collateral balance is zero.
        if collateral_balance.is_zero() {
            continue;
        }

        let price = dango_oracle::raw_query_price(querier, &denom)?;
        let collateral_value = price.value_of_unit_amount(collateral_balance);
        total_adjusted_collateral_value += collateral_value * power.into_inner();
    }

    if total_adjusted_collateral_value.is_zero() {
        bail!("the account has no collateral");
    }

    // Calculate the utilization rate.
    let utilization_rate = total_debt_value / total_adjusted_collateral_value;

    Ok(HealthResponse {
        utilization_rate,
        total_debt_value,
        total_adjusted_collateral_value,
    })
}

pub fn query_health(ctx: ImmutableCtx) -> anyhow::Result<HealthResponse> {
    let app_cfg: AppConfig = ctx.querier.query_app_config()?;

    // Query all debts for the account.
    let debts = ctx
        .querier
        .query_wasm_raw(app_cfg.addresses.lending, DEBTS.path(ctx.contract))?
        .map(|coins| coins.deserialize_borsh::<Coins>())
        .transpose()?
        .unwrap_or_default();

    calculate_account_health(
        &ctx.querier,
        ctx.contract,
        debts,
        app_cfg.lending.collateral_powers,
    )
}
