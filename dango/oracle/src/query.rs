use {
    crate::PRICE_FEEDS,
    dango_types::oracle::PythId,
    grug::{ImmutableCtx, StdResult},
    pyth_sdk::PriceFeed,
};

pub fn query_price_feed(ctx: ImmutableCtx, id: PythId) -> StdResult<PriceFeed> {
    PRICE_FEEDS.load(ctx.storage, id)
}
