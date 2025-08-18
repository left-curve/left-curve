use {
    dango_types::oracle::{PriceSource, QueryPriceSourcesRequest},
    grug::{Addr, Lengthy, NonEmpty, QuerierExt, QuerierWrapper, Shared, StdResult},
    pyth_client::{PythClient, PythClientCache, PythClientTrait},
    pyth_types::{PriceUpdate, PythId},
    reqwest::IntoUrl,
    std::{
        fmt::Debug,
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
        thread,
    },
    tokio::runtime::Runtime,
    tokio_stream::StreamExt,
    tracing::error,
};

/// Handler for the PythClient to be used in the ProposalPreparer, used to
/// keep all code related to Pyth for PP in a single structure.
pub struct PythHandler<P> {
    client: P,
    shared_vaas: Shared<PriceUpdate>,
    current_ids: Vec<PythId>,
    stoppable_thread: Option<(Arc<AtomicBool>, thread::JoinHandle<()>)>,
}

impl PythHandler<PythClient> {
    pub fn new<U: IntoUrl>(base_url: U) -> PythHandler<PythClient> {
        let shared_vaas = Shared::new(PriceUpdate::Empty);

        Self {
            client: PythClient::new(base_url).unwrap(),
            shared_vaas,
            current_ids: vec![],
            stoppable_thread: None,
        }
    }
}

impl PythHandler<PythClientCache> {
    pub fn new_with_cache<U>(base_url: U) -> PythHandler<PythClientCache>
    where
        U: IntoUrl,
    {
        let shared_vaas = Shared::new(PriceUpdate::Empty);

        Self {
            client: PythClientCache::new(base_url).unwrap(),
            shared_vaas,
            current_ids: vec![],
            stoppable_thread: None,
        }
    }
}

impl<P> PythHandler<P>
where
    P: PythClientTrait,
{
    pub fn fetch_latest_price_update(&self) -> PriceUpdate {
        // Retrieve the VAAs from the shared memory and consume them in order to
        // avoid pushing the same VAAs again.
        self.shared_vaas.replace(PriceUpdate::Empty)
    }

    pub fn close_stream(&mut self) {
        if let Some((keep_running, _handle)) = self.stoppable_thread.take() {
            keep_running.store(false, Ordering::SeqCst);
        }

        // Closing any potentially connected earlier stream.
        self.client.close();
    }

    //  TODO: optimize this by using the raw WasmScan query.
    /// Retrieve the Pyth ids from the Oracle contract.
    pub fn pyth_ids(querier: QuerierWrapper, oracle: Addr) -> StdResult<Vec<PythId>> {
        let new_ids = querier
            .query_wasm_smart(oracle, QueryPriceSourcesRequest {
                start_after: None,
                limit: Some(u32::MAX),
            })?
            .into_values()
            .filter_map(|price_source| {
                if let PriceSource::Pyth { id, .. } = price_source {
                    Some(id)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        Ok(new_ids)
    }
}

impl<P> PythHandler<P>
where
    P: PythClientTrait + Send + 'static,
    P::Error: Debug,
{
    fn connect_stream<I>(&mut self, ids: NonEmpty<I>)
    where
        I: IntoIterator + Lengthy + Send + Clone + 'static,
        I::Item: ToString,
    {
        self.close_stream();

        let shared_vaas = self.shared_vaas.clone();
        let keep_running = Arc::new(AtomicBool::new(true));
        let mut client = self.client.clone();

        self.stoppable_thread = Some((
            keep_running.clone(),
            thread::spawn(move || {
                let rt = match Runtime::new() {
                    Ok(rt) => rt,
                    Err(err) => {
                        error!(error = err.to_string(), "Failed to create Tokio runtime");
                        keep_running.store(false, Ordering::SeqCst);
                        return;
                    },
                };

                rt.block_on(async {
                    let mut stream = match client.stream(ids).await {
                        Ok(stream) => stream,
                        Err(err) => {
                            error!(error = err.to_string(), "Failed to create Pyth stream");
                            keep_running.store(false, Ordering::SeqCst);
                            return;
                        },
                    };

                    loop {
                        tokio::select! {
                            _ = tokio::time::sleep(tokio::time::Duration::from_millis(500)) => {
                                if !keep_running.load(Ordering::Relaxed) {
                                    return;
                                }
                            }

                            data = stream.next() => {
                                if !keep_running.load(Ordering::Acquire) {
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

    pub fn update_stream(&mut self, querier: QuerierWrapper, oracle: Addr) -> StdResult<()> {
        // Retrieve the Pyth ids from the Oracle contract.
        let pyth_ids = Self::pyth_ids(querier, oracle)?;

        // Load the state of streaming.
        let is_stream_running = if let Some((keep_running, _)) = self.stoppable_thread.as_ref() {
            keep_running.load(Ordering::Acquire)
        } else {
            false
        };

        // If stream is closed and there are no PythIds, we can return early.
        if !is_stream_running && pyth_ids.is_empty() {
            return Ok(());
        }

        // If the stream is running and the PythIds are the same, we can return early.
        if is_stream_running && self.current_ids == pyth_ids {
            return Ok(());
        }

        // The PythIds have changed or the streaming is closed unexpectedly.
        self.current_ids = pyth_ids.clone();

        self.close_stream();

        if let Ok(pyth_ids) = NonEmpty::new(pyth_ids) {
            self.connect_stream(pyth_ids);
        }

        Ok(())
    }
}
