use {
    grug_testing::setup_tracing_subscriber,
    pyth_lazer_client::{client::PythLazerClientBuilder, ws_connection::AnyResponse},
    pyth_lazer_protocol::{
        router::{
            Channel, DeliveryFormat, FixedRate, Format, JsonBinaryEncoding, PriceFeedId,
            PriceFeedProperty, SubscriptionParams, SubscriptionParamsRepr,
        },
        subscription::{Response, SubscribeRequest, SubscriptionId},
    },
    pyth_types::constants::LAZER_ACCESS_TOKEN_TEST,
    std::str::FromStr,
    tracing::{Level, info},
    url::Url,
};

// TODO: remove this test once lazer implementation is completed.
#[ignore = "remove once the lazer implementation is completed"]
#[tokio::test]
async fn test() {
    setup_tracing_subscriber(Level::INFO);
    let builder =
        PythLazerClientBuilder::new(LAZER_ACCESS_TOKEN_TEST.to_string()).with_endpoints(vec![
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

    let subscribe_request2 = SubscribeRequest {
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
    client.subscribe(subscribe_request2).await.unwrap();
    info!("Subscription request sent, waiting for responses...");

    let subscribe_request = SubscribeRequest {
        subscription_id: SubscriptionId(2),
        params: SubscriptionParams::new(SubscriptionParamsRepr {
            price_feed_ids: vec![PriceFeedId(18)],
            properties: vec![PriceFeedProperty::Price],
            formats: vec![Format::LeEcdsa],
            delivery_format: DeliveryFormat::Binary,
            json_binary_encoding: JsonBinaryEncoding::Base64,
            parsed: false,
            channel: Channel::FixedRate(FixedRate::RATE_200_MS),
            ignore_invalid_feed_ids: true,
        })
        .unwrap(),
    };

    client.subscribe(subscribe_request).await.unwrap();
    info!("Subscription request sent, waiting for responses...");

    // When I process the data, I need to clear old data.

    while let Some(response) = receiver.recv().await {
        // Drain all the channel.
        // let mut count = 0;
        // loop {
        //     match receiver.try_recv() {
        //         Ok(resp) => {
        //             response = resp;
        //             count += 1;
        //         },
        //         Err(_) => break,
        //     }
        // }
        // info!("Received {} responses in this batch", count);

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
                Response::Subscribed(resp) => {
                    info!("Received subscription response: {:#?}", resp);
                },

                Response::SubscriptionError(resp) => {
                    info!("Received subscription error: {:#?}", resp);
                },
                _ => {
                    info!("Received non-stream update response: {:#?}", resp);
                },
            },
            AnyResponse::Binary(update) => {
                info!("Subscription ID: {}", update.subscription_id.0);
                // sleep(Duration::from_millis(20)).await;
                // for msg in &update.messages {
                //     match msg {
                //         Message::LeEcdsa(lecdsa_msg) => {
                //             let binary = Binary::from_inner(lecdsa_msg.payload.clone());
                //             let decoded = binary.deserialize_json::<JsonUpdate>().unwrap();
                //             info!("Decoded ECDSA message: {:#?}", decoded);
                //         },
                //         _ => {
                //             info!("Received non-ECDSA message: {:#?}", msg);
                //         },
                //     }
                // }
            },
        }
        // info!("{:#?}", response);
    }
}
