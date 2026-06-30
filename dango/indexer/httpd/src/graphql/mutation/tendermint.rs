use {
    async_graphql::*,
    dango_primitives::{BroadcastTxOutcome, HttpRequestDetails, Tx},
};

#[derive(Default, Debug)]
pub struct TendermintMutation {}

#[Object]
impl TendermintMutation {
    async fn broadcast_tx_sync(
        &self,
        ctx: &async_graphql::Context<'_>,
        #[graphql(desc = "Transaction as JSON")] tx: Tx,
    ) -> Result<BroadcastTxOutcome, Error> {
        let app_ctx = ctx.data::<crate::context::FullContext>()?;
        let http_request_details = ctx.data::<HttpRequestDetails>()?;

        crate::broadcast::broadcast_tx(app_ctx, http_request_details, tx)
            .await
            .map_err(|e| Error::new(e.to_string()))
    }
}
