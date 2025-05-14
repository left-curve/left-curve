use {
    crate::{PythClient, PythClientTrait},
    grug::NonEmpty,
    pyth_types::constants::{BTC_USD_ID, ETH_USD_ID, PYTH_URL},
    tokio_stream::StreamExt,
    tracing::info,
};

pub async fn cli_pyth_stream() {
    let mut pyth_client = PythClient::new(PYTH_URL).unwrap();
    let mut stream = pyth_client
        .stream(NonEmpty::new_unchecked(vec![BTC_USD_ID, ETH_USD_ID]))
        .await
        .unwrap();

    loop {
        let Some(data) = stream.next().await else {
            continue;
        };

        info!("Read data: {:?}", data);
    }
}
