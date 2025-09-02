use {
    async_graphql::*,
    grug_types::{BroadcastTxOutcome, Inner, JsonSerExt, Tx},
    sentry::configure_scope,
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
        let app_ctx = ctx.data::<crate::context::Context>()?;

        #[cfg(feature = "tracing")]
        tracing::info!(
            sender = %tx.sender.to_string(),
            tx_hash = %tx.tx_hash()?,
            username = tx
                .data
                .get("username")
                .and_then(|v| v.as_str()),
            "`broadcast_tx_sync` called",
        );

        match app_ctx.consensus_client.broadcast_tx(tx.clone()).await {
            Ok(response) => Ok(response),
            Err(e) => {
                #[cfg(feature = "tracing")]
                tracing::error!(error = ?e, tx = ?tx, "`broadcast_tx_sync` failed");

                let tx = tx.to_json_value()?.into_inner();
                configure_scope(|scope| {
                    // NOTE: Sentry might truncate data if too large.
                    scope.set_extra("transaction", tx);
                });

                Err(e.into())
            },
        }
    }
}
