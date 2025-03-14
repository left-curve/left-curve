use {
    dango_types::oracle::{PriceSource, QueryPriceSourcesRequest},
    grug::{Addr, Binary, Lengthy, NonEmpty, QuerierExt, QuerierWrapper, StdResult},
    grug_app::Shared,
    pyth_client::{client_cache::PythClientCache, PythClient, PythClientTrait},
    pyth_types::PythId,
    reqwest::IntoUrl,
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
pub struct PythPPHandler<P> {
    client: P,
    shared_vaas: Shared<Vec<Binary>>,
    current_ids: Vec<PythId>,
    stoppable_thread: Option<(Arc<AtomicBool>, thread::JoinHandle<()>)>,
}

impl PythPPHandler<PythClient> {
    pub fn new<U: IntoUrl>(base_url: U) -> PythPPHandler<PythClient> {
        let shared_vaas = Shared::new(vec![]);

        Self {
            client: PythClient::new(base_url).unwrap(),
            shared_vaas,
            current_ids: vec![],
            stoppable_thread: None,
        }
    }
}

impl PythPPHandler<PythClientCache> {
    #[allow(dead_code)]
    pub fn new_with_cache<U: IntoUrl>(base_url: U) -> PythPPHandler<PythClientCache> {
        let shared_vaas = Shared::new(vec![]);

        Self {
            client: PythClientCache::new(base_url).unwrap(),
            shared_vaas,
            current_ids: vec![],
            stoppable_thread: None,
        }
    }
}

impl<P> PythPPHandler<P>
where
    P: PythClientTrait,
{
    pub fn fetch_latest_vaas(&self) -> Vec<Binary> {
        // Retrieve the VAAs from the shared memory and consume them in order to
        // avoid pushing the same VAAs again.
        self.shared_vaas.replace(vec![])
    }

    pub fn close_stream(&mut self) {
        if let Some((keep_running, _handle)) = self.stoppable_thread.take() {
            keep_running.store(false, Ordering::Relaxed);
        }

        // Closing any potentially connected earlier stream.
        self.client.close();
    }

    /// Retrieve the Pyth ids from the Oracle contract.
    pub fn pyth_ids(querier: QuerierWrapper, oracle: Addr) -> StdResult<Vec<PythId>> {
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

        Ok(new_ids)
    }
}

impl<P> PythPPHandler<P>
where
    P: PythClientTrait + Send + 'static,
    P::Error: std::fmt::Debug,
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

    pub fn update_stream(&mut self, querier: QuerierWrapper, oracle: Addr) -> StdResult<()> {
        // Retrieve the Pyth ids from the Oracle contract.
        let pyth_ids = Self::pyth_ids(querier, oracle)?;

        // If the ids are the same, do nothing.
        if pyth_ids == self.current_ids {
            return Ok(());
        }

        // The PythIds have changed; close the previous stream and establish a new one.
        self.current_ids = pyth_ids.clone();

        self.close_stream();

        if let Ok(pyth_ids) = NonEmpty::new(pyth_ids) {
            self.connect_stream(pyth_ids);
        }

        Ok(())
    }
}
