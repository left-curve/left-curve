use {
    crate::graphql::types::tendermint::{TxAsyncResponse, TxCommitResponse, TxSyncResponse},
    async_graphql::*,
    base64::{Engine, engine::general_purpose::STANDARD},
    sentry::configure_scope,
    tendermint_rpc::Client,
};

#[derive(Default, Debug)]
pub struct TendermintMutation {}

#[Object]
impl TendermintMutation {
    async fn broadcast_tx_sync(
        &self,
        ctx: &async_graphql::Context<'_>,
        #[graphql(desc = "The base64 encoded transaction to broadcast")] tx: String,
    ) -> Result<TxSyncResponse, Error> {
        let app_ctx = ctx.data::<crate::context::Context>()?;

        let http_client = tendermint_rpc::HttpClient::new(app_ctx.tendermint_endpoint.as_str())?;

        let tx_bytes = STANDARD.decode(tx.clone())?;

        match http_client.broadcast_tx_sync(tx_bytes).await {
            Ok(response) => Ok(response.into()),
            Err(e) => {
                #[cfg(feature = "tracing")]
                {
                    tracing::error!("broadcast_tx_sync failed: {e:?}");
                    tracing::error!("transaction: {:?}", &tx);
                }

                configure_scope(|scope| {
                    // NOTE: sentry might truncate data if too large
                    scope.set_extra("transaction", tx.into());
                });

                Err(e.into())
            },
        }
    }

    async fn broadcast_tx_async(
        &self,
        ctx: &async_graphql::Context<'_>,
        #[graphql(desc = "The base64 encoded transaction to broadcast")] tx: String,
    ) -> Result<TxAsyncResponse, Error> {
        let app_ctx = ctx.data::<crate::context::Context>()?;

        let http_client = tendermint_rpc::HttpClient::new(app_ctx.tendermint_endpoint.as_str())?;

        let tx_bytes = STANDARD.decode(tx.clone())?;

        match http_client.broadcast_tx_async(tx_bytes).await {
            Ok(response) => Ok(response.into()),
            Err(e) => {
                #[cfg(feature = "tracing")]
                {
                    tracing::error!("broadcast_tx_async failed: {e:?}");
                    tracing::error!("transaction: {:?}", &tx);
                }

                configure_scope(|scope| {
                    // NOTE: sentry might truncate data if too large
                    scope.set_extra("transaction", tx.into());
                });

                Err(e.into())
            },
        }
    }

    async fn broadcast_tx_commit(
        &self,
        ctx: &async_graphql::Context<'_>,
        #[graphql(desc = "The base64 encoded transaction to broadcast")] tx: String,
    ) -> Result<TxCommitResponse, Error> {
        let app_ctx = ctx.data::<crate::context::Context>()?;

        let http_client = tendermint_rpc::HttpClient::new(app_ctx.tendermint_endpoint.as_str())?;

        let tx_bytes = STANDARD.decode(tx.clone())?;

        match http_client.broadcast_tx_commit(tx_bytes).await {
            Ok(response) => Ok(response.into()),
            Err(e) => {
                #[cfg(feature = "tracing")]
                {
                    tracing::error!("broadcast_tx_commit failed: {e:?}");
                    tracing::error!("transaction: {:?}", &tx);
                }

                configure_scope(|scope| {
                    // NOTE: sentry might truncate data if too large
                    scope.set_extra("transaction", tx.into());
                });

                Err(e.into())
            },
        }
    }
}
