use {
    crate::graphql::types::transfer::Transfer,
    async_graphql::{
        futures_util::{stream::Stream, StreamExt},
        *,
    },
};

#[derive(Default)]
pub struct TransferSubscription;

#[Subscription]
impl TransferSubscription {
    async fn transfer<'a>(&self, ctx: &Context<'a>) -> Result<impl Stream<Item = Transfer> + 'a> {
        let app_ctx = ctx.data::<indexer_httpd::context::Context>()?;

        Ok(app_ctx
            .pubsub
            .subscribe_block_minted()
            .map(|block_height| Transfer {
                block_height,
                ..Default::default()
            }))
    }
}
