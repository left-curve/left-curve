use {
    dango_types::oracle::{PriceSource, QueryPriceSourcesRequest},
    grug::{Addr, Binary, Lengthy, NonEmpty, QuerierExt, QuerierWrapper, StdResult},
    grug_app::Shared,
    pyth_client::{middleware_cache::PythMiddlewareCache, PythClient, PythClientTrait},
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
};
/// Handler for the PythClient to be used in the ProposalPreparer, used to
/// keep all code related to Pyth for PP in a single structure.
pub struct PythClientPPHandler<P> {
    client: P,
    shared_vaas: Shared<Vec<Binary>>,
    current_ids: Vec<PythId>,
    stoppable_thread: Option<(Arc<AtomicBool>, thread::JoinHandle<()>)>,
}

impl PythClientPPHandler<PythClient> {
    pub fn new<S: ToString>(base_url: S) -> PythClientPPHandler<PythClient> {
        let shared_vaas = Shared::new(vec![]);

        Self {
            client: PythClient::new(base_url),
            shared_vaas,
            current_ids: vec![],
            stoppable_thread: None,
        }
    }
}

impl PythClientPPHandler<PythMiddlewareCache> {
    #[allow(dead_code)]
    pub fn new_with_cache<S: ToString>(base_url: S) -> PythClientPPHandler<PythMiddlewareCache> {
        let shared_vaas = Shared::new(vec![]);

        Self {
            client: PythMiddlewareCache::new(base_url),
            shared_vaas,
            current_ids: vec![],
            stoppable_thread: None,
        }
    }
}

impl<P> PythClientPPHandler<P>
where
    P: PythClientTrait + Send + 'static,
    P::Error: std::fmt::Debug,
{
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
        // let base_url = self.client.base_url.clone();

        let keep_running = Arc::new(AtomicBool::new(true));

        let client = self.client.clone();

        self.stoppable_thread = Some((
            keep_running.clone(),
            thread::spawn(move || {
                let rt = Runtime::new().unwrap();
                rt.block_on(async {
                    let mut stream = client.stream(ids).await.unwrap();

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
