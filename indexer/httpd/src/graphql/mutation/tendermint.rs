use {
    async_graphql::*,
    grug_types::{BroadcastTxOutcome, HttpRequestDetails, Inner, JsonSerExt, Tx},
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
        let http_request_details = ctx.data::<HttpRequestDetails>()?;

        app_ctx
            .sql_context
            .transaction_hash_details
            .lock()
            .map_err(|e| Error::new(format!("Failed to lock transaction_hash_details: {e}")))?
            .insert(tx.tx_hash()?.to_string(), http_request_details.clone());

        #[cfg(feature = "tracing")]
        tracing::info!(
            sender = %tx.sender.to_string(),
            tx_hash = %tx.tx_hash()?,
            remote_ip = ?http_request_details.remote_ip,
            peer_ip = ?http_request_details.peer_ip,
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
