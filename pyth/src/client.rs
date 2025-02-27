use {
    dango_app::LatestVaaResponse,
    grug::{JsonDeExt, StdResult},
    reqwest::Client,
    std::sync::mpsc::{self, Receiver, Sender},
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
    ) -> StdResult<Receiver<LatestVaaResponse>> {
        let (tx, rx) = mpsc::channel::<LatestVaaResponse>();

        let base_url = self.base_url.clone();
        self.thread = Some(tokio::spawn(PythClient::run_streaming_inner(
            base_url, ids, tx,
        )));

        Ok(rx)
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
        tx: Sender<LatestVaaResponse>,
    ) {
        loop {
            let mut stream = Client::new()
                .get(format!("{}/v2/updates/price/stream", base_url))
                .query(&ids)
                .query(&[("parsed", "false")])
                .query(&[("encoding", "base64")])
                .send()
                .await
                .unwrap()
                .bytes_stream();

            println!("Connesso al server SSE!");

            let mut buffer = Vec::new();
            while let Some(chunk) = stream.next().await {
                match chunk {
                    Ok(bytes) => {
                        buffer.extend_from_slice(&bytes);

                        // Prova a trovare un delimitatore di evento (\n\n) nel buffer
                        while let Some(pos) = find_event_delimiter(&buffer) {
                            let mut event_data = buffer.drain(..pos).collect::<Vec<u8>>(); // Estrai i dati dell'evento
                            buffer.drain(..2); // Rimuovi il delimitatore (\n\n) dal buffer

                            // Check for "data: " prefix
                            if !event_data.starts_with(b"data:") {
                                println!("Event data does not start with 'data: '");
                                continue;
                            }

                            // Remove the "data: " prefix.
                            event_data.drain(0..5);

                            // Try to deserialize the event data.
                            if let Ok(vaas) = event_data.deserialize_json::<LatestVaaResponse>() {
                                // TODO: add info
                                tx.send(vaas).unwrap();
                            } else {
                                // TODO: add error
                                println!("Error deserializing event data");
                            }
                        }
                    },
                    Err(e) => eprintln!("Errore nel flusso: {}", e),
                }
            }
        }
    }
}

fn find_event_delimiter(buffer: &[u8]) -> Option<usize> {
    buffer.windows(2).position(|window| window == b"\n\n")
}
