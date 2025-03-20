use {
    crate::{PythClientTrait, error},
    async_stream::stream,
    async_trait::async_trait,
    grug::{Binary, Inner, JsonDeExt, Lengthy, NonEmpty},
    pyth_types::LatestVaaResponse,
    reqwest::{Client, IntoUrl, Url},
    reqwest_eventsource::{Event, EventSource, retry::ExponentialBackoff},
    std::{
        pin::Pin,
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
        time::Duration,
    },
    tokio_stream::StreamExt,
    tracing::{debug, error},
};

/// PythClient is a client to interact with the Pyth network.
#[derive(Debug, Clone)]
pub struct PythClient {
    pub base_url: Url,
    keep_running: Arc<AtomicBool>,
}

impl PythClient {
    pub fn new<U: IntoUrl>(base_url: U) -> Result<Self, error::Error> {
        Ok(Self {
            base_url: base_url.into_url()?,
            keep_running: Arc::new(AtomicBool::new(false)),
        })
    }

    fn create_request_params<I>(ids: NonEmpty<I>) -> Vec<(&'static str, String)>
    where
        I: IntoIterator + Lengthy,
        I::Item: ToString,
    {
        let mut params = ids
            .into_inner()
            .into_iter()
            .map(|id| ("ids[]", id.to_string()))
            .collect::<Vec<_>>();

        params.push(("parsed", "false".to_string()));
        params.push(("encoding", "base64".to_string()));
        // If, for some reason, the price id is invalid, ignore it
        // instead of making the request fail.
        // TODO: remove? The request still fail with random id. To be ignored,
        // the id must respect the correct id format (len, string in hex).
        params.push(("ignore_invalid_price_ids", "true".to_string()));

        params
    }
}

#[async_trait]
impl PythClientTrait for PythClient {
    type Error = crate::error::Error;

    async fn stream<I>(
        &mut self,
        ids: NonEmpty<I>,
    ) -> Result<Pin<Box<dyn tokio_stream::Stream<Item = Vec<Binary>> + Send>>, Self::Error>
    where
        I: IntoIterator + Lengthy + Send + Clone,
        I::Item: ToString,
    {
        // Close the previous connection.
        self.close();

        let params = PythClient::create_request_params(ids);
        let builder = Client::new()
            .get(self.base_url.join("v2/updates/price/stream")?)
            .query(&params);

        // Connect to EventSource.
        let mut es = EventSource::new(builder)?;

        // Set the exponential backoff for reconnect.
        es.set_retry_policy(Box::new(ExponentialBackoff::new(
            Duration::from_secs(1),
            1.5,
            Some(Duration::from_secs(30)),
            None,
        )));

        self.keep_running = Arc::new(AtomicBool::new(true));
        let keep_running = self.keep_running.clone();

        let stream = stream! {
            loop {
                tokio::select! {
                    // Safety check if, for some reason, the stream needs to be closed
                    // but there are no more events incoming.
                    _ = tokio::time::sleep(tokio::time::Duration::from_millis(1000)) => {
                        if !keep_running.load(Ordering::Relaxed) {
                            return;
                        }
                    }

                    data = es.next() => {
                        if !keep_running.load(Ordering::Acquire) {
                            return;
                        }

                        if let Some(event) = data{
                            match event {
                                Ok(Event::Open) => debug!("Pyth SSE connection open"),
                                Ok(Event::Message(message)) => {
                                    match message.data.deserialize_json::<LatestVaaResponse>() {
                                        Ok(vaas) => {
                                            yield vaas.binary.data;
                                        },
                                        Err(err) => {
                                            error!(
                                                err = err.to_string(),
                                                "Failed to deserialize Pyth event into LatestVaaResponse"
                                            );
                                        },
                                    }
                                },
                                Err(err) => {
                                    error!(
                                        err = err.to_string(),
                                        "Error while receiving the events from Pyth"
                                    );
                                    es.close();
                                },
                            }
                        }

                    }

                }
            }
        };

        Ok(Box::pin(stream))
    }

    fn get_latest_vaas<I>(&self, ids: NonEmpty<I>) -> Result<Vec<Binary>, Self::Error>
    where
        I: IntoIterator + Clone + Lengthy,
        I::Item: ToString,
    {
        Ok(reqwest::blocking::Client::new()
            .get(self.base_url.join("v2/updates/price/latest")?)
            .query(&PythClient::create_request_params(ids.clone()))
            .send()?
            .error_for_status()?
            .json::<LatestVaaResponse>()?
            .binary
            .data)
    }

    fn close(&mut self) {
        self.keep_running.store(false, Ordering::SeqCst);
    }
}
