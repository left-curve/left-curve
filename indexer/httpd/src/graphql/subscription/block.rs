use {
    crate::graphql::types::block::Block,
    async_graphql::{
        futures_util::{stream::Stream, StreamExt},
        *,
    },
    std::pin::Pin,
};

#[derive(Default)]
pub struct BlockSubscription;

#[Subscription]
impl<'a> BlockSubscription {
    async fn blocks(
        &'a self,
        ctx: &Context<'a>,
    ) -> Result<Pin<Box<dyn Stream<Item = Block> + Send + 'a>>> {
        let app_ctx = ctx.data::<crate::context::Context>()?;

        // TODO: broadcast the last block height to the client when they subscribe

        let stream = app_ctx.pubsub.block_minted().map(|block_height| Block {
            block_height,
            ..Default::default()
        });

        Ok(Box::pin(stream))

        // let rx = app_ctx.pubsub.sender.subscribe();

        // Ok(BroadcastStream::new(rx).filter_map(|item| async move {
        //     let mut block = Block::default();
        //     block.block_height = item.unwrap() as u64;
        //     Some(block)
        // }))

        // let mut value = 0;
        // Ok(
        //     tokio_stream::wrappers::IntervalStream::new(tokio::time::interval(
        //         Duration::from_secs(1),
        //     ))
        //     .map(move |_| {
        //         value += step;

        //         let mut block = Block::default();
        //         block.block_height = value as u64;
        //         block
        //     }),
        // )
    }
}
