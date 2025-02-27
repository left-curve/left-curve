use {
    dango_app::LatestVaaResponse,
    grug::{Binary, JsonDeExt, StdResult},
    grug_app::Shared,
    reqwest::Client,
    reqwest_eventsource::{Event, EventSource},
    tokio::task::JoinHandle,
    tokio_stream::StreamExt,
};

pub struct PythClient {
    base_url: String,
    thread: Option<JoinHandle<()>>,
}

impl PythClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            thread: None,
        }
    }

    /// Start a SSE connection to the Pyth network.
    pub fn run_streaming(
        &mut self,
        ids: Vec<(&'static str, String)>,
    ) -> StdResult<Shared<Vec<Binary>>> {
        let shared = Shared::new(Vec::new());
        let shared_clone = shared.clone();
        let base_url = self.base_url.clone();
        self.thread = Some(tokio::spawn(PythClient::run_streaming_inner(
            base_url,
            ids,
            shared_clone,
        )));

        Ok(shared)
    }

    /// Close the client and stop the streaming thread.
    pub fn close(&mut self) {
        if let Some(thread) = self.thread.take() {
            thread.abort();
        }
    }

    /// Inner function to run the SSE connection.
    async fn run_streaming_inner(
        base_url: String,
        ids: Vec<(&str, String)>,
        shared: Shared<Vec<Binary>>,
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
                    Ok(Event::Open) => println!("Connection Open!"),
                    Ok(Event::Message(message)) => {
                        let vaas = message
                            .data
                            .deserialize_json::<LatestVaaResponse>()
                            .unwrap();
                        shared.write_with(|mut shared_vaas| *shared_vaas = vaas.binary.data);
                    },
                    Err(err) => {
                        println!("Error: {}", err);
                        es.close();
                    },
                }
            }
        }
    }
}
