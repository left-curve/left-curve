use {
    crate::{CONFIG, FEES_BY_USER},
    dango_types::taxman::{Config, FeePayments, FeeType, QueryMsg},
    grug::{
        Addr, Bound, ImmutableCtx, Json, JsonSerExt, Number, Order, StdError, StdResult, Timestamp,
    },
    std::collections::BTreeMap,
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> StdResult<Json> {
    match msg {
        QueryMsg::Config {} => query_config(ctx)?.to_json_value(),
        QueryMsg::FeesForUser {
            user,
            fee_type,
            since,
        } => query_fees_for_user(ctx, user, fee_type, since)?.to_json_value(),
    }
}

fn query_config(ctx: ImmutableCtx) -> StdResult<Config> {
    CONFIG.load(ctx.storage)
}

fn query_fees_for_user(
    ctx: ImmutableCtx,
    user: Addr,
    fee_type: Option<FeeType>,
    since: Option<Timestamp>,
) -> StdResult<FeePayments> {
    let fees_now = FEES_BY_USER
        .prefix(user)
        .values(ctx.storage, None, None, Order::Descending)
        .next()
        .transpose()?
        .unwrap_or(BTreeMap::new());

    let mut coins_now = if let Some(fee_type) = fee_type {
        fees_now.get(&fee_type).cloned().unwrap_or_default()
    } else {
        fees_now
            .values()
            .try_fold(FeePayments::default(), |mut acc, payment| {
                acc.coins.insert_many(payment.coins.clone())?;
                acc.usd_value.checked_add_assign(payment.usd_value)?;
                Ok::<FeePayments, StdError>(acc)
            })?
    };

    let fees_since = if let Some(since) = since {
        FEES_BY_USER
            .prefix(user)
            .values(
                ctx.storage,
                None,
                Some(Bound::Inclusive(since)),
                Order::Descending,
            )
            .next()
            .transpose()?
            .unwrap_or(BTreeMap::new())
    } else {
        BTreeMap::new()
    };

    let coins_since = if let Some(fee_type) = fee_type {
        fees_since.get(&fee_type).cloned().unwrap_or_default()
    } else {
        fees_since
            .values()
            .try_fold(FeePayments::default(), |mut acc, payment| {
                acc.coins.insert_many(payment.coins.clone())?;
                acc.usd_value.checked_add_assign(payment.usd_value)?;
                Ok::<FeePayments, StdError>(acc)
            })?
    };

    Ok(FeePayments {
        coins: coins_now.coins.deduct_many(coins_since.coins)?.clone(),
        usd_value: coins_now.usd_value.checked_sub(coins_since.usd_value)?,
    })
}
