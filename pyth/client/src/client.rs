use {
    grug::{Binary, JsonDeExt, NonEmpty, StdResult},
    grug_app::Shared,
    pyth_types::LatestVaaResponse,
    reqwest::Client,
    reqwest_eventsource::{Event, EventSource},
    std::{
        sync::{
            atomic::{AtomicBool, Ordering},
            Arc,
        },
        thread::{self},
    },
    tokio::runtime::Runtime,
    tokio_stream::StreamExt,
    tracing::{error, info},
};

pub struct PythClient {
    base_url: String,
    keep_running: Arc<AtomicBool>,
}

impl PythClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            keep_running: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Start a SSE connection to the Pyth network.
    /// If shared_vaas is provided, it will update the shared value with the latest VAA.
    /// Otherwise, it will create a new shared value and return it.
    pub fn run_streaming(
        &mut self,
        ids: NonEmpty<Vec<(&'static str, String)>>,
        shared_vaas: Option<Shared<Vec<Binary>>>,
    ) -> StdResult<Shared<Vec<Binary>>> {
        let base_url = self.base_url.clone();

        let shared = shared_vaas.unwrap_or_default();
        let shared_clone = shared.clone();

        // Create a new atomic bool. Don't reuse the old one since there is no
        // guarantee that the old thread has already stopped.
        self.keep_running = Arc::new(AtomicBool::new(true));
        let keep_running_clone = self.keep_running.clone();

        thread::spawn(|| {
            let rt = Runtime::new().unwrap();
            rt.block_on(async {
                PythClient::run_streaming_inner(base_url, ids, shared_clone, keep_running_clone)
                    .await;
                info!("Pyth SSE connection closed");
            });
        });

        Ok(shared)
    }

    /// Close the client and stop the streaming thread.
    pub fn close(&mut self) {
        self.keep_running.store(false, Ordering::SeqCst);
    }

    /// Inner function to run the SSE connection.
    async fn run_streaming_inner(
        base_url: String,
        ids: NonEmpty<Vec<(&str, String)>>,
        shared: Shared<Vec<Binary>>,
        keep_running: Arc<AtomicBool>,
    ) {
        loop {
            let builder = Client::new()
                .get(format!("{}/v2/updates/price/stream", base_url))
                .query(&ids)
                .query(&[("parsed", "false")])
                .query(&[("encoding", "base64")]);

            // Connect to EventSource.
            let mut es = EventSource::new(builder).unwrap();

            // Waiting for next message and send through channel.
            while let Some(event) = es.next().await {
                match event {
                    Ok(Event::Open) => info!("Pyth SSE connection open"),
                    Ok(Event::Message(message)) => {
                        if let Ok(vaas) = message.data.deserialize_json::<LatestVaaResponse>() {
                            // Check if the thread should keep running.
                            if !keep_running.load(Ordering::Relaxed) {
                                return;
                            }

                            shared.write_with(|mut shared_vaas| *shared_vaas = vaas.binary.data);
                        } else {
                            error!("Failed to deserialize the message: {:?}", message);
                        }
                    },
                    Err(err) => {
                        error!("Error: {}", err);
                        es.close();
                    },
                }
            }
        }
    }
}
