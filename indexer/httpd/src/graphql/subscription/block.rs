use {
    crate::graphql::types::block::Block,
    async_graphql::{
        futures_util::{stream::Stream, StreamExt},
        *,
    },
};

#[derive(Default)]
pub struct BlockSubscription;

#[Subscription]
impl BlockSubscription {
    async fn block<'a>(&self, ctx: &Context<'a>) -> Result<impl Stream<Item = Block> + 'a> {
        let app_ctx = ctx.data::<crate::context::Context>()?;

        Ok(app_ctx
            .pubsub
            .subscribe_block_minted()
            .map(|block_height| Block {
                block_height,
                ..Default::default()
            }))
    }
}
