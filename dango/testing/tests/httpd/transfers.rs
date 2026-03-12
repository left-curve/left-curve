use {
    crate::{
        PaginationDirection, Transfers, build_actix_app, call_graphql_query, paginate_transfers,
        transfers_query,
    },
    assertor::*,
    dango_testing::{
        HyperlaneTestSuite, TestOption, create_user_and_account, setup_test_with_indexer,
    },
    dango_types::{account_factory, constants::usdc},
    graphql_client::GraphQLQuery,
    grug::{Addressable, Coins, Message, NonEmpty, ResultExt},
    grug_app::Indexer,
    indexer_client::{SubscribeTransfers, subscribe_transfers},
    indexer_testing::{
        GraphQLCustomRequest, call_ws_graphql_stream, parse_graphql_subscription_response,
    },
    itertools::Itertools,
    tokio::sync::mpsc,
};

#[tokio::test(flavor = "multi_thread")]
async fn graphql_returns_transfer_and_accounts() -> anyhow::Result<()> {
    let (mut suite, mut accounts, _, contracts, _, _, dango_httpd_context, _, _db_guard) =
        setup_test_with_indexer(TestOption::default()).await;

    // Copied from benchmarks.rs
    let msgs = vec![Message::execute(
        contracts.account_factory,
        &account_factory::ExecuteMsg::RegisterAccount {},
        Coins::one(usdc::DENOM.clone(), 100_000_000).unwrap(),
    )?];

    suite
        .send_messages_with_gas(
            &mut accounts.user1,
            50_000_000,
            NonEmpty::new_unchecked(msgs),
        )
        .should_succeed();

    suite.app.indexer.wait_for_finish().await?;

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let variables = transfers_query::Variables {
                    block_height: Some(1),
                    ..Default::default()
                };

                let response = call_graphql_query::<_, transfers_query::ResponseData>(
                    dango_httpd_context,
                    Transfers::build_query(variables),
                )
                .await?;

                assert_that!(response.data).is_some();
                let data = response.data.unwrap();

                assert_that!(data.transfers.nodes).has_length(2);

                assert_that!(
                    data.transfers
                        .nodes
                        .iter()
                        .map(|t| t.block_height)
                        .collect::<Vec<_>>()
                )
                .is_equal_to(vec![1, 1]);

                assert_that!(
                    data.transfers
                        .nodes
                        .iter()
                        .map(|t| t.amount.as_str())
                        .collect::<Vec<_>>()
                )
                .is_equal_to(vec!["100000000", "100000000"]);

                data.transfers.nodes.iter().for_each(|node| {
                    assert!(
                        !node.tx_hash.is_empty(),
                        "Transaction hash should not be empty."
                    );
                });

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread")]
async fn graphql_transfers_with_user_index() -> anyhow::Result<()> {
    let (
        suite,
        mut accounts,
        codes,
        contracts,
        validator_sets,
        _,
        dango_httpd_context,
        _,
        _db_guard,
    ) = setup_test_with_indexer(TestOption::default()).await;

    let mut suite = HyperlaneTestSuite::new(suite, validator_sets, &contracts);

    let mut user1 = create_user_and_account(&mut suite, &mut accounts, &contracts, &codes);
    let user2 = create_user_and_account(&mut suite, &mut accounts, &contracts, &codes);

    suite
        .transfer(
            &mut user1,
            user2.address(),
            Coins::one(usdc::DENOM.clone(), 100).unwrap(),
        )
        .should_succeed();

    suite.app.indexer.wait_for_finish().await?;

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let variables = transfers_query::Variables {
                    user_index: Some(user1.user_index() as i64),
                    ..Default::default()
                };

                let response = call_graphql_query::<_, transfers_query::ResponseData>(
                    dango_httpd_context,
                    Transfers::build_query(variables),
                )
                .await?;

                assert_that!(response.data).is_some();
                let data = response.data.unwrap();

                // We expect two transfers:
                // 1. When creating user1, from Gateway contract to user1's account. (amount: 150_000_000)
                // 2. From user1 to user2. (amount: 100)
                assert_that!(data.transfers.nodes).has_length(2);

                assert_that!(
                    data.transfers
                        .nodes
                        .iter()
                        .map(|t| t.amount.as_str())
                        .collect::<Vec<_>>()
                )
                .is_equal_to(
                    // Transfer 1: 150_000_000 (see the `create_user_and_account` function)
                    // Transfer 2: 100
                    vec!["100", "150000000"],
                );

                assert_that!(
                    data.transfers
                        .nodes
                        .iter()
                        .filter_map(|t| t
                            .from_account
                            .as_ref()
                            .and_then(|a| a.users.first())
                            .map(|u| u.user_index))
                        .collect::<Vec<_>>()
                )
                .is_equal_to(
                    // Transfer 1: sender is Gateway contract, which doesn't have a user index
                    // Transfer 2: sender is user1
                    vec![user1.user_index() as i64],
                );

                assert_that!(
                    data.transfers
                        .nodes
                        .iter()
                        .filter_map(|t| t
                            .to_account
                            .as_ref()
                            .and_then(|a| a.users.first())
                            .map(|u| u.user_index))
                        .collect::<Vec<_>>()
                )
                .is_equal_to(
                    // Transfer 1: recipient is user1
                    // Transfer 2: recipient is user2
                    vec![user2.user_index() as i64, user1.user_index() as i64],
                );

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread")]
async fn graphql_transfers_with_wrong_user_index() -> anyhow::Result<()> {
    let (
        suite,
        mut accounts,
        codes,
        contracts,
        validator_sets,
        _,
        dango_httpd_context,
        _,
        _db_guard,
    ) = setup_test_with_indexer(TestOption::default()).await;

    let mut suite = HyperlaneTestSuite::new(suite, validator_sets, &contracts);

    let mut user1 = create_user_and_account(&mut suite, &mut accounts, &contracts, &codes);
    let user2 = create_user_and_account(&mut suite, &mut accounts, &contracts, &codes);

    suite
        .transfer(
            &mut user1,
            user2.address(),
            Coins::one(usdc::DENOM.clone(), 100).unwrap(),
        )
        .should_succeed();

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let variables = transfers_query::Variables {
                    user_index: Some(114514), // a random user index that doesn't exist
                    ..Default::default()
                };

                let response = call_graphql_query::<_, transfers_query::ResponseData>(
                    dango_httpd_context,
                    Transfers::build_query(variables),
                )
                .await?;

                assert_that!(response.data).is_some();
                let data = response.data.unwrap();

                assert_that!(data.transfers.nodes).is_empty();

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread")]
async fn graphql_paginate_transfers() -> anyhow::Result<()> {
    let (mut suite, mut accounts, _, _, _, _, dango_httpd_context, _, _db_guard) =
        setup_test_with_indexer(TestOption::default()).await;

    // Create 10 transfers to paginate through
    for recipient in [
        accounts.owner.address(),
        accounts.user1.address(),
        accounts.user2.address(),
        accounts.user3.address(),
        accounts.user4.address(),
        accounts.user5.address(),
        accounts.user6.address(),
        accounts.user7.address(),
        accounts.user8.address(),
        accounts.user9.address(),
    ] {
        suite
            .transfer(
                &mut accounts.user1,
                recipient,
                Coins::one(usdc::DENOM.clone(), 100_000_000).unwrap(),
            )
            .should_succeed();
    }

    suite.app.indexer.wait_for_finish().await?;

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let page_size = 2;

                // 1. first with descending order
                let block_heights: Vec<_> = paginate_transfers(
                    dango_httpd_context.clone(),
                    page_size,
                    transfers_query::Variables {
                        sort_by: Some(transfers_query::TransferSortBy::BLOCK_HEIGHT_DESC),
                        ..Default::default()
                    },
                    PaginationDirection::Forward,
                )
                .await?
                .into_iter()
                .map(|n| n.block_height)
                .collect();

                assert_that!(block_heights.iter().copied().unique().collect::<Vec<_>>())
                    .is_equal_to((1i64..=10).rev().collect::<Vec<_>>());

                // 2. first with ascending order
                let block_heights: Vec<_> = paginate_transfers(
                    dango_httpd_context.clone(),
                    page_size,
                    transfers_query::Variables {
                        sort_by: Some(transfers_query::TransferSortBy::BLOCK_HEIGHT_ASC),
                        ..Default::default()
                    },
                    PaginationDirection::Forward,
                )
                .await?
                .into_iter()
                .map(|n| n.block_height)
                .collect();

                assert_that!(block_heights.iter().copied().unique().collect::<Vec<_>>())
                    .is_equal_to((1i64..=10).collect::<Vec<_>>());

                // 3. last with descending order
                let block_heights: Vec<_> = paginate_transfers(
                    dango_httpd_context.clone(),
                    page_size,
                    transfers_query::Variables {
                        sort_by: Some(transfers_query::TransferSortBy::BLOCK_HEIGHT_DESC),
                        ..Default::default()
                    },
                    PaginationDirection::Backward,
                )
                .await?
                .into_iter()
                .map(|n| n.block_height)
                .collect();

                assert_that!(block_heights.iter().copied().unique().collect::<Vec<_>>())
                    .is_equal_to((1i64..=10).collect::<Vec<_>>());

                // 4. last with ascending order
                let block_heights: Vec<_> = paginate_transfers(
                    dango_httpd_context.clone(),
                    page_size,
                    transfers_query::Variables {
                        sort_by: Some(transfers_query::TransferSortBy::BLOCK_HEIGHT_ASC),
                        ..Default::default()
                    },
                    PaginationDirection::Backward,
                )
                .await?
                .into_iter()
                .map(|n| n.block_height)
                .collect();

                assert_that!(block_heights.iter().copied().unique().collect::<Vec<_>>())
                    .is_equal_to((1i64..=10).rev().collect::<Vec<_>>());

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread")]
async fn graphql_subscribe_to_transfers() -> anyhow::Result<()> {
    let (mut suite, mut accounts, _, contracts, _, _, dango_httpd_context, _, _db_guard) =
        setup_test_with_indexer(TestOption::default()).await;

    // Copied from benchmarks.rs
    let msgs = vec![Message::execute(
        contracts.account_factory,
        &account_factory::ExecuteMsg::RegisterAccount {},
        Coins::one(usdc::DENOM.clone(), 100_000_000).unwrap(),
    )?];

    suite
        .send_messages_with_gas(
            &mut accounts.user1,
            50_000_000,
            NonEmpty::new_unchecked(msgs),
        )
        .should_succeed();

    suite.app.indexer.wait_for_finish().await?;

    // Use typed subscription from indexer-client
    let request_body = GraphQLCustomRequest::from_query_body(
        SubscribeTransfers::build_query(subscribe_transfers::Variables::default()),
        "transfers",
    );

    let local_set = tokio::task::LocalSet::new();

    // Can't call this from LocalSet so using channels instead.
    let (crate_block_tx, mut rx) = mpsc::channel::<u32>(1);
    tokio::spawn(async move {
        while let Some(_idx) = rx.recv().await {
            let msgs = vec![Message::transfer(
                accounts.user2.address(),
                Coins::one(usdc::DENOM.clone(), 123).unwrap(),
            )?];

            suite
                .send_messages_with_gas(
                    &mut accounts.user1,
                    50_000_000,
                    NonEmpty::new_unchecked(msgs),
                )
                .should_succeed();
        }
        Ok::<(), anyhow::Error>(())
    });

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let name = request_body.name;
                let (_srv, _ws, mut framed) =
                    call_ws_graphql_stream(dango_httpd_context, build_actix_app, request_body)
                        .await?;

                // 1st response is always the existing last block
                let response = parse_graphql_subscription_response::<
                    Vec<subscribe_transfers::SubscribeTransfersTransfers>,
                >(&mut framed, name)
                .await?;

                assert_that!(
                    response
                        .data
                        .into_iter()
                        .map(|t| t.block_height)
                        .collect::<Vec<_>>()
                )
                .is_equal_to(vec![1i64, 1]);

                crate_block_tx.send(2).await.unwrap();

                // 2nd response
                let response = parse_graphql_subscription_response::<
                    Vec<subscribe_transfers::SubscribeTransfersTransfers>,
                >(&mut framed, name)
                .await?;

                assert_that!(
                    response
                        .data
                        .into_iter()
                        .map(|t| t.block_height)
                        .collect::<Vec<_>>()
                )
                .is_equal_to(vec![2i64]);

                crate_block_tx.send(3).await.unwrap();

                // 3rd response
                let response = parse_graphql_subscription_response::<
                    Vec<subscribe_transfers::SubscribeTransfersTransfers>,
                >(&mut framed, name)
                .await?;

                assert_that!(
                    response
                        .data
                        .into_iter()
                        .map(|t| t.block_height)
                        .collect::<Vec<_>>()
                )
                .is_equal_to(vec![3i64]);

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread")]
async fn graphql_subscribe_to_transfers_with_filter() -> anyhow::Result<()> {
    let (mut suite, mut accounts, _, contracts, _, _, dango_httpd_context, _, _db_guard) =
        setup_test_with_indexer(TestOption::default()).await;

    // Copied from benchmarks.rs
    let msgs = vec![Message::execute(
        contracts.account_factory,
        &account_factory::ExecuteMsg::RegisterAccount {},
        Coins::one(usdc::DENOM.clone(), 100_000_000).unwrap(),
    )?];

    suite
        .send_messages_with_gas(
            &mut accounts.user1,
            50_000_000,
            NonEmpty::new_unchecked(msgs),
        )
        .should_succeed();

    // Use typed subscription from indexer-client
    let request_body = GraphQLCustomRequest::from_query_body(
        SubscribeTransfers::build_query(subscribe_transfers::Variables {
            address: Some(accounts.user1.address().to_string()),
            ..Default::default()
        }),
        "transfers",
    );

    let local_set = tokio::task::LocalSet::new();

    // Can't call this from LocalSet so using channels instead.
    let (create_block_tx, mut rx) = mpsc::channel::<u32>(1);
    tokio::spawn(async move {
        while let Some(_idx) = rx.recv().await {
            // Copied from benchmarks.rs
            let msgs = vec![Message::execute(
                contracts.account_factory,
                &account_factory::ExecuteMsg::RegisterAccount {},
                Coins::one(usdc::DENOM.clone(), 100_000_000).unwrap(),
            )?];

            suite
                .send_messages_with_gas(
                    &mut accounts.user1,
                    50_000_000,
                    NonEmpty::new_unchecked(msgs),
                )
                .should_succeed();
        }
        Ok::<(), anyhow::Error>(())
    });

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let name = request_body.name;
                let (_srv, _ws, mut framed) =
                    call_ws_graphql_stream(dango_httpd_context, build_actix_app, request_body)
                        .await?;

                // 1st response is always the existing last block
                let response = parse_graphql_subscription_response::<
                    Vec<subscribe_transfers::SubscribeTransfersTransfers>,
                >(&mut framed, name)
                .await?;

                // 1st transfer because we filter on one address
                assert_that!(
                    response
                        .data
                        .into_iter()
                        .map(|t| t.amount)
                        .collect::<Vec<_>>()
                )
                .is_equal_to(vec!["100000000".to_string()]);

                create_block_tx.send(2).await.unwrap();

                // 2nd response
                let response = parse_graphql_subscription_response::<
                    Vec<subscribe_transfers::SubscribeTransfersTransfers>,
                >(&mut framed, name)
                .await?;

                assert_that!(
                    response
                        .data
                        .into_iter()
                        .map(|t| t.amount)
                        .collect::<Vec<_>>()
                )
                .is_equal_to(vec!["100000000".to_string()]);

                create_block_tx.send(3).await.unwrap();

                // 3rd response
                let response = parse_graphql_subscription_response::<
                    Vec<subscribe_transfers::SubscribeTransfersTransfers>,
                >(&mut framed, name)
                .await?;

                assert_that!(
                    response
                        .data
                        .into_iter()
                        .map(|t| t.amount)
                        .collect::<Vec<_>>()
                )
                .is_equal_to(vec!["100000000".to_string()]);

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}
