use {
    assertor::*,
    dango_indexer_sql::entity,
    dango_testing::{
        TestOption,
        perps::{create_perps_fill, pair_id, setup_perps_env},
        setup_test_with_indexer,
    },
    grug_app::Indexer,
    sea_orm::EntityTrait,
};

#[tokio::test(flavor = "multi_thread")]
async fn index_perps_events() -> anyhow::Result<()> {
    let (mut suite, mut accounts, _, contracts, _, _, dango_context, _, _db_guard) =
        setup_test_with_indexer(TestOption::default()).await;

    setup_perps_env(&mut suite, &mut accounts, &contracts, 2_000, 100_000);

    create_perps_fill(&mut suite, &mut accounts, &contracts, &pair_id(), 2_000, 5);

    suite.app.indexer.wait_for_finish().await?;

    let events = entity::perps_events::Entity::find()
        .all(&dango_context.db)
        .await?;

    // All events should have correct pair_id and non-empty fields.
    for event in &events {
        assert_that!(event.pair_id.as_str()).is_equal_to(pair_id().to_string().as_str());
        assert!(!event.user_addr.is_empty(), "user_addr should not be empty");
        assert!(!event.data.is_null(), "data should not be null");
    }

    // A fill produces order_filled events (one per side).
    let filled: Vec<_> = events
        .iter()
        .filter(|e| e.event_type == "order_filled")
        .collect();
    // At least 2 OrderFilled events (maker + taker).
    assert_that!(filled.len()).is_at_least(2);

    Ok(())
}
