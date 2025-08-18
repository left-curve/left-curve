use {
    grug_testing::setup_tracing_subscriber,
    pyth_lazer_client::{client::PythLazerClientBuilder, ws_connection::AnyResponse},
    pyth_lazer_protocol::{
        router::{
            Channel, DeliveryFormat, Format, JsonBinaryEncoding, PriceFeedId, PriceFeedProperty,
            SubscriptionParams, SubscriptionParamsRepr,
        },
        subscription::{Response, SubscribeRequest, SubscriptionId},
    },
    std::{str::FromStr, time::Duration},
    tokio::time::sleep,
    tracing::{Level, info},
    url::Url,
};

#[ignore = "work in progress"]
#[tokio::test]
async fn test() {
    setup_tracing_subscriber(Level::INFO);
    let builder =
        PythLazerClientBuilder::new("gr6DS1uhFL7dUrcrboueU4ykRk2XhOfT3GO-demo".to_string())
            .with_endpoints(vec![
                Url::from_str("wss://pyth-lazer-0.dourolabs.app/v1/stream").unwrap(),
            ]);

    info!("Connecting to Pyth Lazer server...");
    let mut client = builder.build().unwrap();

    let mut receiver = client.start().await.unwrap();
    info!("Client started, waiting for subscription...");

    let subscribe_request = SubscribeRequest {
        subscription_id: SubscriptionId(1),
        params: SubscriptionParams::new(SubscriptionParamsRepr {
            price_feed_ids: vec![PriceFeedId(1), PriceFeedId(2)],
            properties: vec![PriceFeedProperty::Price],
            formats: vec![Format::LeEcdsa],
            delivery_format: DeliveryFormat::Binary,
            json_binary_encoding: JsonBinaryEncoding::Base64,
            parsed: false,
            channel: Channel::RealTime,
            ignore_invalid_feed_ids: true,
        })
        .unwrap(),
    };

    client.subscribe(subscribe_request).await.unwrap();
    info!("Subscription request sent, waiting for responses...");

    // When I process the data, I need to clear old data.

    while let Some(mut response) = receiver.recv().await {
        // Drain all the channel.
        let mut count = 0;
        loop {
            match receiver.try_recv() {
                Ok(resp) => {
                    response = resp;
                    count += 1;
                },
                Err(_) => break,
            }
        }
        info!("Received {} responses in this batch", count);

        match &response {
            AnyResponse::Json(resp) => match resp {
                Response::StreamUpdated(data) => {
                    let payload = data.payload.clone();

                    // if let Some(data_ecdsa) = payload.le_ecdsa {
                    //     info!("Received ECDSA data raw: {:#?}", data_ecdsa.data);

                    //     let data = Binary::from_str(&data_ecdsa.data)
                    //         .unwrap()
                    //         .deserialize_json::<ParsedPayload>();

                    //     info!("Received ECDSA data: {:#?}", data);
                    // }

                    if let Some(parsed) = payload.parsed {
                        info!("Received parsed data: {:#?}", parsed);
                    };
                },
                _ => {
                    info!("Received non-stream update response: {:#?}", resp);
                },
            },
            AnyResponse::Binary(_) => {
                info!("Received binary update");
                sleep(Duration::from_millis(20)).await;
                // for msg in &update.messages {
                // match msg {
                //     Message::LeEcdsa(lecdsa_msg) => {
                //         let binary = Binary::from_inner(lecdsa_msg.payload.clone());
                //         let decoded = binary.deserialize_json::<JsonUpdate>().unwrap();
                //         info!("Decoded ECDSA message: {:#?}", decoded);
                //     },
                //     _ => {
                //         info!("Received non-ECDSA message: {:#?}", msg);
                //     },
                // }
                // }
            },
        }
        // info!("{:#?}", response);
    }
}
