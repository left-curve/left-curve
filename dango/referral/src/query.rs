use {
    crate::{REFEREE, REFEREE_COUNT},
    dango_types::{account_factory::UserIndex, referral::QueryMsg},
    grug::{ImmutableCtx, Json, JsonSerExt, StdResult},
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> anyhow::Result<Json> {
    match msg {
        QueryMsg::Referrer { referee_index } => {
            let res = referrer(ctx, referee_index)?;
            res.to_json_value()
        },
        QueryMsg::RefereeCount { user } => {
            let res = referee_count(ctx, user);
            res.to_json_value()
        },
    }
    .map_err(Into::into)
}

fn referrer(ctx: ImmutableCtx, referee_index: UserIndex) -> StdResult<UserIndex> {
    REFEREE.load(ctx.storage, referee_index)
}

fn referee_count(ctx: ImmutableCtx, user: UserIndex) -> u32 {
    REFEREE_COUNT.load(ctx.storage, user).unwrap_or(0)
}
