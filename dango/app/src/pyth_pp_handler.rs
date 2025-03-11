use {
    dango_types::oracle::{PriceSource, QueryPriceSourcesRequest},
    grug::{Addr, Binary, Lengthy, NonEmpty, QuerierExt, QuerierWrapper, StdResult},
    grug_app::Shared,
    pyth_client::PythClient,
    pyth_types::PythId,
    std::{
        sync::{
            atomic::{AtomicBool, Ordering},
            Arc,
        },
        thread,
    },
    tokio::runtime::Runtime,
    tokio_stream::StreamExt,
    tracing::warn,
};

/// Handler for the PythClient to be used in the ProposalPreparer, used to
/// keep all code related to Pyth for PP in a single structure.
pub struct PythClientPPHandler {
    client: PythClient,
    shared_vaas: Shared<Vec<Binary>>,
    current_ids: Vec<PythId>,
    stoppable_thread: Option<(Arc<AtomicBool>, thread::JoinHandle<()>)>,
}

impl PythClientPPHandler {
    // So tighten to hermes pyth than I don't see the need for multiple endpoints?
    pub fn new<S: ToString>(base_url: S) -> Self {
        // Creating it here
        let shared_vaas = Shared::new(vec![]);

        let client = if cfg!(test) {
            warn!("Running in test mode");
            PythClient::new(base_url).with_middleware_cache()
        } else {
            PythClient::new(base_url)
        };

        Self {
            client,
            shared_vaas,
            current_ids: vec![],
            stoppable_thread: None,
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

        self.current_ids = new_ids.clone();

        // Check if the ids are the same.
        if self.current_ids == new_ids {
            return Ok(());
        }

        if let Ok(ids) = NonEmpty::new(new_ids) {
            self.connect_stream(ids);
        }

        Ok(())
    }

    fn connect_stream<I>(&mut self, ids: NonEmpty<I>)
    where
        I: IntoIterator + Lengthy + Send + Clone + 'static,
        I::Item: ToString,
    {
        // Closing any potentially connected earlier stream
        self.client.close();

        if let Some((keep_running, _handle)) = self.stoppable_thread.take() {
            keep_running.store(false, Ordering::Relaxed);
            // If we wanted to wait for the thread to finish, but we don't care.
            // handle.join().unwrap();
        }

        let shared_vaas = self.shared_vaas.clone();
        let base_url = self.client.base_url.clone();

        let keep_running = Arc::new(AtomicBool::new(true));

        self.stoppable_thread = Some((
            keep_running.clone(),
            thread::spawn(move || {
                let rt = Runtime::new().unwrap();
                rt.block_on(async {
                    let mut stream = PythClient::stream(&base_url, ids).await.unwrap();

                    loop {
                        tokio::select! {
                            _ = tokio::time::sleep(tokio::time::Duration::from_millis(500)) => {
                                if !keep_running.load(Ordering::Relaxed) {
                                    return;
                                }
                            }

                            data = stream.next() => {
                                if !keep_running.load(Ordering::Relaxed) {
                                    return;
                                }

                                if let Some(data) = data {
                                    shared_vaas.write_with(|mut shared_vaas| *shared_vaas = data);
                                }
                            }

                        }
                    }
                });
            }),
        ));
    }

    pub fn fetch_latest_vaas(&self) -> Vec<Binary> {
        // Retrieve the VAAs from the shared memory and consume them in order to
        // avoid pushing the same VAAs again.
        self.shared_vaas.replace(vec![])
    }
}
