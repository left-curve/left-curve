use {
    crate::context::FullContext,
    anyhow::anyhow,
    dango_primitives::{BroadcastTxOutcome, HttpRequestDetails, Inner, JsonSerExt, Tx},
    sentry::configure_scope,
};

/// Broadcast a signed transaction to the mempool via the consensus client.
///
/// Records the requester's HTTP details in the cache context (so the indexer
/// can associate them with the transaction later), then submits the tx. A
/// mempool rejection (non-zero `check_tx.code`) comes back as `Ok`; only a
/// transport failure to the consensus client is `Err`.
///
/// Shared by the GraphQL `broadcastTxSync` mutation and the REST `POST
/// /broadcast` handler.
pub async fn broadcast_tx(
    app_ctx: &FullContext,
    details: &HttpRequestDetails,
    tx: Tx,
) -> anyhow::Result<BroadcastTxOutcome> {
    // Store HTTP request details for this transaction in the cache context.
    // This is used later by the indexer to associate HTTP request details with
    // transactions.
    app_ctx
        .indexer_cache_context
        .transactions_http_request_details
        .lock()
        .map_err(|e| anyhow!("failed to lock transactions_http_request_details: {e}"))?
        .insert(tx.tx_hash()?, details.clone());

    #[cfg(feature = "tracing")]
    tracing::info!(
        sender = %tx.sender.to_string(),
        tx_hash = %tx.tx_hash()?,
        remote_ip = ?details.remote_ip,
        peer_ip = ?details.peer_ip,
        username = tx.data.get("username").and_then(|v| v.as_str()),
        "`broadcast_tx` called",
    );

    app_ctx
        .consensus_client
        .broadcast_tx(tx.clone())
        .await
        .inspect_err(|_e| {
            #[cfg(feature = "tracing")]
            tracing::error!(error = ?_e, tx = ?tx, "`broadcast_tx` failed");

            // Best-effort: attach the tx to the Sentry scope. A serialization
            // failure here must not mask the original broadcast error.
            if let Ok(tx) = tx.to_json_value() {
                configure_scope(|scope| {
                    // NOTE: Sentry might truncate data if too large.
                    scope.set_extra("transaction", tx.into_inner());
                });
            }
        })
}
