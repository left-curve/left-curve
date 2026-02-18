use {
    crate::{CONFIG, VOLUMES_BY_USER},
    dango_types::{
        account_factory::UserIndex,
        taxman::{Config, QueryMsg},
    },
    grug::{
        Bound, ImmutableCtx, Json, JsonSerExt, Number, NumberConst, Order, StdResult, Timestamp,
        Udec128_6,
    },
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> anyhow::Result<Json> {
    match msg {
        QueryMsg::Config {} => {
            let res = query_config(ctx)?;
            res.to_json_value()
        },
        QueryMsg::VolumeByUser { user, since } => {
            let res = query_volume_by_user(ctx, user, since)?;
            res.to_json_value()
        },
    }
    .map_err(Into::into)
}

fn query_config(ctx: ImmutableCtx) -> StdResult<Config> {
    CONFIG.load(ctx.storage)
}

fn query_volume_by_user(
    ctx: ImmutableCtx,
    user: UserIndex,
    since: Option<Timestamp>,
) -> anyhow::Result<Udec128_6> {
    let volume_now = VOLUMES_BY_USER
        .prefix(user)
        .values(ctx.storage, None, None, Order::Descending)
        .next()
        .transpose()?
        .unwrap_or(Udec128_6::ZERO);

    let volume_since = if let Some(since) = since {
        VOLUMES_BY_USER
            .prefix(user)
            .values(
                ctx.storage,
                None,
                Some(Bound::Inclusive(since)),
                Order::Descending,
            )
            .next()
            .transpose()?
            .unwrap_or(Udec128_6::ZERO)
    } else {
        Udec128_6::ZERO
    };

    Ok(volume_now.checked_sub(volume_since)?)
}
