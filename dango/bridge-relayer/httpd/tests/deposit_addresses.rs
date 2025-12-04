use {
    actix_web::{App, HttpResponse, http::StatusCode, test, web},
    chrono::Utc,
    dango_bridge_relayer_httpd::{
        context::Context,
        entity, migrations,
        routes::{self},
    },
    dango_types::bitcoin::{Config, MultisigSettings, Network},
    grug::{__private::hex_literal::hex, Addr, HexByteArray, NonEmpty, Uint128, btree_set},
    metrics_exporter_prometheus::PrometheusBuilder,
    sea_orm::{ColumnTrait, Database, EntityTrait, QueryFilter},
    sea_orm_migration::MigratorTrait,
    std::{collections::HashSet, time::Duration},
};

async fn test_context() -> Context {
    let pk1 = hex!("029ba1aeddafb6ff65d403d50c0db0adbb8b5b3616c3bc75fb6fecd075327099f6");
    let bridge_config = Config {
        network: Network::Testnet,
        vault: "0x0000000000000000000000000000000000000000".to_string(),
        multisig: MultisigSettings::new(
            1,
            NonEmpty::new(btree_set!(HexByteArray::from_inner(pk1))).unwrap(),
        )
        .unwrap(),
        sats_per_vbyte: Uint128::new(1),
        fee_rate_updater: Addr::mock(0),
        minimum_deposit: Uint128::new(1),
        max_output_per_tx: 1,
    };

    let db = Database::connect("sqlite::memory:").await.unwrap();

    migrations::Migrator::up(&db, None).await.unwrap();

    Context::new(bridge_config, db)
}

#[actix_web::test]
async fn test_deposit_addresses() {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    let context = test_context().await;
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(context.clone()))
            .service(routes::deposit_address)
            .service(routes::deposit_addresses),
    )
    .await;

    let metrics_handler = PrometheusBuilder::new().install_recorder().unwrap();
    let metrics_app = test::init_service(App::new().route(
        "/metrics",
        web::get().to(move || {
            let metrics_handler = metrics_handler.clone();
            metrics_handler.run_upkeep();
            async move {
                let metrics = metrics_handler.render();

                HttpResponse::Ok()
                    .content_type("text/plain; version=0.0.4")
                    .body(metrics)
            }
        }),
    ))
    .await;

    // Try to call without any data in the database. Should return an empty array.
    let req = test::TestRequest::get()
        .uri("/deposit-addresses")
        .to_request();
    let resp = test::call_service(&app, req).await;
    let result = test::read_body(resp).await;
    let res = serde_json::from_slice::<Vec<String>>(&result).unwrap();
    assert_eq!(res.len(), 0);

    // Create 10 deposit addresses.
    let mut addresses = HashSet::<String>::new();
    for i in 0..10 {
        let req = test::TestRequest::post()
            .uri(format!("/deposit-address/{}", Addr::mock(i).to_string()).as_str())
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let result = test::read_body(resp).await;
        let text = String::from_utf8(result.to_vec()).unwrap();
        assert_eq!(text.len(), 62);
        assert!(addresses.insert(text));
    }

    // Try to fetch the metrics. Should contain the total number of deposit addresses created.
    let req = test::TestRequest::get().uri("/metrics").to_request();
    let resp = test::call_service(&metrics_app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let result = test::read_body(resp).await;
    let metrics = String::from_utf8(result.to_vec()).unwrap();
    assert!(metrics.contains("http_bridge_relayer_deposit_address_total 10"));

    // Try to fetch all the deposit addresses. Should work.
    let req = test::TestRequest::get()
        .uri("/deposit-addresses")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let result = test::read_body(resp).await;
    let res = serde_json::from_slice::<Vec<String>>(&result).unwrap();
    assert_eq!(res.len(), 10);
    assert!(res.iter().all(|addr| addresses.contains(addr)));

    // Get the current timestamp in milliseconds.
    let now = Utc::now().timestamp_millis();

    // Sleep for 1 second.
    tokio::time::sleep(Duration::from_secs(1)).await;

    // Create one more deposit address
    let req = test::TestRequest::post()
        .uri(format!("/deposit-address/{}", Addr::mock(10).to_string()).as_str())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let result = test::read_body(resp).await;
    let text = String::from_utf8(result.to_vec()).unwrap();
    assert_eq!(text.len(), 62);

    // Try to fetch the deposit addresses after the saved timestamp. Should return only the new address.
    let req = test::TestRequest::get()
        .uri(format!("/deposit-addresses?after_created_at={}", now).as_str())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let result = test::read_body(resp).await;
    let res = serde_json::from_slice::<Vec<String>>(&result).unwrap();
    assert_eq!(res.len(), 1);
    assert_eq!(res[0], text);

    // Check created_at timestamp of the new deposit address.
    let created_at = entity::deposit_address::Entity::find()
        .filter(entity::deposit_address::Column::Address.eq(text))
        .one(&context.db)
        .await
        .unwrap()
        .unwrap()
        .created_at;
    assert!(created_at > now);

    // Try to create an existing deposit address. Should update the created_at timestamp and return the same address.
    let req = test::TestRequest::post()
        .uri(format!("/deposit-address/{}", Addr::mock(10).to_string()).as_str())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let result = test::read_body(resp).await;
    let text = String::from_utf8(result.to_vec()).unwrap();
    assert_eq!(text, text);
    assert_eq!(text.len(), 62);

    // Try to fetch the deposit addresses after the saved timestamp. Should return only the new address.
    let req = test::TestRequest::get()
        .uri(format!("/deposit-addresses?after_created_at={}", now).as_str())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let result = test::read_body(resp).await;
    let res = serde_json::from_slice::<Vec<String>>(&result).unwrap();
    assert_eq!(res.len(), 1);
    assert_eq!(res[0], text);

    // Ensure the created_at timestamp of the new deposit address is updated.
    let created_at_new = entity::deposit_address::Entity::find()
        .filter(entity::deposit_address::Column::Address.eq(text))
        .one(&context.db)
        .await
        .unwrap()
        .unwrap()
        .created_at;
    assert!(created_at_new > created_at);
}
