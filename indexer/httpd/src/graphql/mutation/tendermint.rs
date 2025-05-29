use {
    async_graphql::*,
    grug_math::Inner,
    grug_types::{JsonDeExt, JsonSerExt, Tx},
    sentry::configure_scope,
};

#[derive(Default, Debug)]
pub struct TendermintMutation {}

#[Object]
impl TendermintMutation {
    async fn broadcast_tx_sync(
        &self,
        ctx: &async_graphql::Context<'_>,
        #[graphql(desc = "Transaction as JSON")] tx: serde_json::Value,
    ) -> Result<serde_json::Value, Error> {
        let app_ctx = ctx.data::<crate::context::Context>()?;

        let decoded_tx: Tx = tx.to_string().deserialize_json()?;

        match app_ctx
            .consensus_client
            .broadcast_tx(decoded_tx.clone())
            .await
        {
            Ok(response) => Ok(response.to_json_value()?.into_inner()),
            Err(e) => {
                #[cfg(feature = "tracing")]
                tracing::error!(error = ?e, tx = ?decoded_tx, "broadcast_tx_sync failed");

                configure_scope(|scope| {
                    // NOTE: Sentry might truncate data if too large.
                    scope.set_extra("transaction", tx);
                });

                Err(e.into())
            },
        }
    }
}
