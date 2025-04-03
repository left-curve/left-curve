mod common_function;

use {
    axum::{
        Router,
        extract::State,
        response::sse::{Event, Sse},
        routing::get,
        serve,
    },
    common_function::{test_latest_vaas, test_stream},
    futures::stream::{self, Stream},
    grug::{JsonSerExt, NonEmpty, setup_tracing_subscriber},
    pyth_client::{PythClient, PythClientCache, PythClientTrait},
    pyth_types::{
        ATOM_USD_ID, BNB_USD_ID, BTC_USD_ID, ETH_USD_ID, LatestVaaBinaryResponse,
        LatestVaaResponse, PYTH_URL,
    },
    std::{
        convert::Infallible,
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
        time::Duration,
    },
    tokio::{net::TcpListener, time::interval},
    tokio_stream::StreamExt,
    tracing::{info, warn},
};

#[ignore = "rely on network calls"]
#[test]
fn latest_vaas_network() {
    let pyth_client = PythClient::new(PYTH_URL).unwrap();
    test_latest_vaas(pyth_client, vec![BTC_USD_ID, ETH_USD_ID]);
}

#[ignore = "rely on network calls"]
#[tokio::test]
async fn test_sse_stream() {
    let client = PythClient::new(PYTH_URL).unwrap();
    test_stream(client, vec![BTC_USD_ID, ETH_USD_ID], vec![
        ATOM_USD_ID,
        BNB_USD_ID,
    ])
    .await;
}

#[tokio::test]
async fn test_client_reconnection() {
    let mut client = PythClient::new("http://127.0.0.1:3030").unwrap();
    setup_tracing_subscriber(tracing::Level::DEBUG);
    let mut stream = client
        .stream(NonEmpty::new_unchecked(vec![BTC_USD_ID]))
        .await
        .unwrap();

    // Start client before the server to ensure that the client in able to reconnect.
    tokio::select! {
        _ = tokio::time::sleep(Duration::from_secs(3)) => (),
        _ = stream.next() => {
            panic!("Stream should be empty")
        },
    }

    start_server().await;

    // Read some data from the stream.
    // During this pull, the client will receive:
    // - valid data;
    // - invalid data;
    // - connection close;
    // Each time the client should be able to reconnect.
    for _ in 0..10 {
        let _ = stream.next().await.unwrap();
    }
}

async fn start_server() {
    let counter = Arc::new(AtomicUsize::new(0));
    let app = Router::new()
        .route("/v2/updates/price/stream", get(sse_handler))
        .with_state(counter.clone());

    let listener = TcpListener::bind("127.0.0.1:3030").await.unwrap();

    tokio::spawn(async move {
        serve(listener, app.into_make_service()).await.unwrap();
    });
}

async fn sse_handler(
    State(counter): State<Arc<AtomicUsize>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let pyth_client_cache = PythClientCache::new(PYTH_URL).unwrap();

    // Create the data to send to the client.
    let mut values = vec![];
    for _ in 0..4 {
        let latest_vaas = pyth_client_cache
            .get_latest_vaas(NonEmpty::new_unchecked(vec![BTC_USD_ID]))
            .unwrap();
        let data = LatestVaaResponse {
            binary: LatestVaaBinaryResponse { data: latest_vaas },
        };

        match data.to_json_value() {
            Ok(data) => values.push(data),
            Err(err) => info!(
                error = err.to_string(),
                "Error creating json from LatestVaaResponse"
            ),
        }
    }

    let request_index = counter.clone().fetch_add(1, Ordering::SeqCst);

    // Add invalid string in the values.
    values.insert(2, "{}".to_json_value().unwrap());

    let stream = stream::unfold(
        (
            0u32,
            interval(Duration::from_secs(1)),
            values,
            request_index,
        ),
        |(mut count, mut int, values, request_index)| async move {
            int.tick().await;

            let json = if let Some(json) = values.get(count as usize) {
                info!("Sending data to the client");
                count += 1;
                json.clone()
            } else {
                if request_index == 0 {
                    // Panic to simulate an unexpected disconnection.
                    warn!("Panic inside the server");
                    panic!("BOOM 💥");
                }
                // None ti simulate a connection close.
                warn!("Closing server connection");
                return None;
            };

            Some((
                Ok(Event::default().json_data(json).unwrap()),
                (count, int, values, request_index),
            ))
        },
    );

    Sse::new(stream)
}
