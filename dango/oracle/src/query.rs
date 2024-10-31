use {
    crate::PRICE_SOURCES,
    dango_types::oracle::PrecisionedPrice,
    grug::{Denom, ImmutableCtx},
};

pub fn query_price(ctx: ImmutableCtx, denom: Denom) -> anyhow::Result<PrecisionedPrice> {
    PRICE_SOURCES
        .load(ctx.storage, &denom)?
        .get_price(ctx.storage)
}
