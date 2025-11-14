use {
    grug::{Lengthy, NonEmpty, Shared},
    pyth_client::{PythClient, PythClientCache, PythClientTrait},
    pyth_types::{PriceUpdate, PythLazerSubscriptionDetails},
    reqwest::IntoUrl,
    std::{
        fmt::Debug,
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
        thread,
        time::Duration,
    },
    tokio::{runtime::Runtime, time::sleep},
    tokio_stream::StreamExt,
    tracing::error,
};

/// Number of attempts to connect to the Pyth stream before giving up.
const CONNECT_ATTEMPTS: usize = 3;

/// Handler for the PythClient to be used in the ProposalPreparer, used to
/// keep all code related to Pyth for PP in a single structure.
pub struct PythHandler<P>
where
    P: PythClientTrait,
{
    client: P,
    ids: NonEmpty<Vec<PythLazerSubscriptionDetails>>,
    shared_vaas: Shared<Option<PriceUpdate>>,
    stoppable_thread: Option<(Arc<AtomicBool>, thread::JoinHandle<()>)>,
}

impl PythHandler<PythClient> {
    pub fn new<V, U, T>(
        endpoints: NonEmpty<V>,
        access_token: T,
        ids: NonEmpty<Vec<PythLazerSubscriptionDetails>>,
    ) -> PythHandler<PythClient>
    where
        V: IntoIterator<Item = U> + Lengthy,
        U: IntoUrl,
        T: ToString,
    {
        Self::new_with_client(PythClient::new(endpoints, access_token).unwrap(), ids)
    }
}

impl PythHandler<PythClientCache> {
    pub fn new_with_cache<V, U, T>(
        endpoints: NonEmpty<V>,
        access_token: T,
        ids: NonEmpty<Vec<PythLazerSubscriptionDetails>>,
    ) -> PythHandler<PythClientCache>
    where
        V: IntoIterator<Item = U> + Lengthy,
        U: IntoUrl,
        T: ToString,
    {
        Self::new_with_client(PythClientCache::new(endpoints, access_token).unwrap(), ids)
    }
}

impl<P> PythHandler<P>
where
    P: PythClientTrait,
{
    fn new_with_client(
        client: P,
        ids: NonEmpty<Vec<PythLazerSubscriptionDetails>>,
    ) -> PythHandler<P> {
        Self {
            client,
            ids,
            shared_vaas: Shared::new(None),
            stoppable_thread: None,
        }
    }

    pub fn fetch_latest_price_update(&self) -> Option<PriceUpdate> {
        // Retrieve the VAAs from the shared memory and consume them in order to
        // avoid pushing the same VAAs again.
        self.shared_vaas.replace(None)
    }

    pub fn close_stream(&mut self) {
        if let Some((keep_running, _handle)) = self.stoppable_thread.take() {
            keep_running.store(false, Ordering::SeqCst);
        }

        // Closing any potentially connected earlier stream.
        self.client.close();
    }
}

impl<P> PythHandler<P>
where
    P: PythClientTrait + Send + 'static,
    P::Error: Debug,
{
    pub fn connect_stream(&mut self) {
        self.close_stream();

        let ids = self.ids.clone();
        let shared_data = self.shared_vaas.clone();
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

                rt.block_on(async move {
                    let mut attempts = 0;

                    // Try to create the stream, retrying up to CONNECT_ATTEMPTS times if it fails.
                    let mut stream = loop {
                        match client.stream(ids.clone()).await {
                            Ok(stream) => {
                                break stream;
                            }
                            Err(err) => {
                                attempts += 1;

                                if attempts < CONNECT_ATTEMPTS {
                                    error!(error = err.to_string(), "Failed to create Pyth stream; attempts: {attempts}");
                                    sleep(Duration::from_millis(100)).await;
                                } else {
                                    error!("Failed to create Pyth stream after {attempts} attempts, stop retrying");
                                    keep_running.store(false, Ordering::SeqCst);
                                    return;
                                }
                            },
                        };
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
                                    shared_data.write_with(|mut shared_vaas| *shared_vaas = Some(data));
                                }
                            }
                        }
                    }
                });
            }),
        ));
    }
}
