use {
    crate::PRICE_SOURCES,
    dango_types::oracle::{PrecisionedPrice, QueryMsg},
    grug::{Denom, ImmutableCtx, Json, JsonSerExt},
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> anyhow::Result<Json> {
    match msg {
        QueryMsg::QueryPrice { denom } => {
            let res = query_price(ctx, denom)?;
            Ok(res.to_json_value()?)
        },
    }
}

pub fn query_price(ctx: ImmutableCtx, denom: Denom) -> anyhow::Result<PrecisionedPrice> {
    PRICE_SOURCES
        .load(ctx.storage, &denom)?
        .get_price(ctx.storage)
}
