use {
    dango_types::oracle::{PriceSource, QueryPriceSourcesRequest},
    grug::{Addr, Binary, NonEmpty, QuerierExt, QuerierWrapper, StdResult},
    grug_app::Shared,
    pyth_client::PythClient,
    pyth_types::PythId,
};

/// Handler for the PythClient to be used in the ProposalPreparer, used to
/// keep all code related to Pyth for PP in a single structure.
pub struct PythClientPPHandler {
    client: PythClient,
    shared_vaas: Shared<Vec<Binary>>,
    old_ids: Vec<PythId>,
}

impl PythClientPPHandler {
    pub fn new(base_url: impl Into<String>) -> Self {
        let client = PythClient::new(base_url);
        let shared_vaas = Shared::new(vec![]);

        Self {
            client,
            shared_vaas,
            old_ids: vec![],
        }
    }

    /// Check if the pyth ids stored on oracle contract are changed; if so, update the Pyth connection.
    pub fn update_ids(&mut self, querier: QuerierWrapper, oracle: Addr) -> StdResult<()> {
        // TODO: optimize this by using the raw WasmScan query.
        let new_ids = querier
            .query_wasm_smart(oracle, QueryPriceSourcesRequest {
                start_after: None,
                limit: Some(u32::MAX),
            })?
            .into_values()
            .filter_map(|price_source| {
                // For now there is only Pyth as PriceSource, but there could be more.
                #[allow(irrefutable_let_patterns)]
                if let PriceSource::Pyth { id, .. } = price_source {
                    Some(id)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        // Check if the ids are the same.
        if self.old_ids == new_ids {
            return Ok(());
        }

        // Otherwise, update the ids and start a new connection to the Pyth network.
        self.old_ids = new_ids.clone();

        // Close the previous connection.
        self.client.close();

        // Start a new connection only if there are some params.
        if let Ok(ids) = NonEmpty::new(new_ids) {
            self.shared_vaas = self.client.run_streaming(ids);
        }

        Ok(())
    }

    pub fn fetch_latest_vaas(&self) -> Vec<Binary> {
        // Retrieve the VAAs from the shared memory and consume them in order to
        // avoid pushing the same VAAs again.
        self.shared_vaas.write_with(|mut prices_lock| {
            let prices = prices_lock.clone();
            *prices_lock = vec![];
            prices
        })
    }
}
