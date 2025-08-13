use {
    grug_testing::setup_tracing_subscriber,
    pyth_lazer_client::{client::PythLazerClientBuilder, ws_connection::AnyResponse},
    pyth_lazer_protocol::{
        router::{
            Channel, DeliveryFormat, FixedRate, Format, JsonBinaryEncoding, PriceFeedId,
            PriceFeedProperty, SubscriptionParams, SubscriptionParamsRepr,
        },
        subscription::{SubscribeRequest, SubscriptionId},
    },
    std::str::FromStr,
    tracing::{Level, info},
    url::Url,
};

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
            delivery_format: DeliveryFormat::Json,
            json_binary_encoding: JsonBinaryEncoding::Base64,
            parsed: false,
            channel: Channel::FixedRate(FixedRate::RATE_200_MS),
            ignore_invalid_feed_ids: true,
        })
        .unwrap(),
    };

    client.subscribe(subscribe_request).await.unwrap();
    info!("Subscription request sent, waiting for responses...");

    while let Some(response) = receiver.recv().await {
        match &response {
            AnyResponse::Json(resp) => {
                info!("Received response: {:#?}", resp);
            },
            AnyResponse::Binary(update) => {
                info!("Received binary update: {:#?}", update);
            },
        }
        info!("{:#?}", response);
    }
}
