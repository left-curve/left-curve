use {
    crate::build_actix_app,
    assertor::*,
    dango_indexer_sql::entity,
    dango_testing::{
        HyperlaneTestSuite, TestOption, create_user_and_account, setup_test_with_indexer,
    },
    dango_types::{
        account::single,
        account_factory::{self, AccountParams},
        constants::usdc,
    },
    graphql_client::{GraphQLQuery, Response},
    grug::{Addressable, Coins, Message, NonEmpty, ResultExt},
    grug_app::Indexer,
    indexer_client::{Transfers, transfers},
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
        &account_factory::ExecuteMsg::RegisterAccount {
            params: AccountParams::Single(single::Params::new(accounts.user1.user_index())),
        },
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
                let variables = transfers::Variables {
                    after: None,
                    before: None,
                    first: None,
                    last: None,
                    sort_by: None,
                    block_height: Some(1),
                    from_address: None,
                    to_address: None,
                    user_index: None,
                };

                let request_body = Transfers::build_query(variables);

                let app = build_actix_app(dango_httpd_context);
                let app = actix_web::test::init_service(app).await;

                let request = actix_web::test::TestRequest::post()
                    .uri("/graphql")
                    .set_json(&request_body)
                    .to_request();

                let response = actix_web::test::call_and_read_body(&app, request).await;
                let response: Response<transfers::ResponseData> =
                    serde_json::from_slice(&response)?;

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
                let variables = transfers::Variables {
                    after: None,
                    before: None,
                    first: None,
                    last: None,
                    sort_by: None,
                    block_height: None,
                    from_address: None,
                    to_address: None,
                    user_index: Some(user1.user_index() as i64),
                };

                let request_body = Transfers::build_query(variables);

                let app = build_actix_app(dango_httpd_context);
                let app = actix_web::test::init_service(app).await;

                let request = actix_web::test::TestRequest::post()
                    .uri("/graphql")
                    .set_json(&request_body)
                    .to_request();

                let response = actix_web::test::call_and_read_body(&app, request).await;
                let response: Response<transfers::ResponseData> =
                    serde_json::from_slice(&response)?;

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
                            .map(|u| u.user_index as u64))
                        .collect::<Vec<_>>()
                )
                .is_equal_to(
                    // Transfer 1: sender is Gateway contract, which doesn't have a user index
                    // Transfer 2: sender is user1
                    vec![user1.user_index() as u64],
                );

                assert_that!(
                    data.transfers
                        .nodes
                        .iter()
                        .filter_map(|t| t
                            .to_account
                            .as_ref()
                            .and_then(|a| a.users.first())
                            .map(|u| u.user_index as u64))
                        .collect::<Vec<_>>()
                )
                .is_equal_to(
                    // Transfer 1: recipient is user1
                    // Transfer 2: recipient is user2
                    vec![user2.user_index() as u64, user1.user_index() as u64],
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
                let variables = transfers::Variables {
                    after: None,
                    before: None,
                    first: None,
                    last: None,
                    sort_by: None,
                    block_height: None,
                    from_address: None,
                    to_address: None,
                    user_index: Some(114514), // a random user index that doesn't exist
                };

                let request_body = Transfers::build_query(variables);

                let app = build_actix_app(dango_httpd_context);
                let app = actix_web::test::init_service(app).await;

                let request = actix_web::test::TestRequest::post()
                    .uri("/graphql")
                    .set_json(&request_body)
                    .to_request();

                let response = actix_web::test::call_and_read_body(&app, request).await;
                let response: Response<transfers::ResponseData> =
                    serde_json::from_slice(&response)?;

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
                let transfers_count = 2;

                // Helper to paginate through all transfers
                async fn paginate_all_transfers(
                    httpd_context: dango_httpd::context::Context,
                    sort_by: transfers::TransferSortBy,
                    first: Option<i64>,
                    last: Option<i64>,
                ) -> anyhow::Result<Vec<i64>> {
                    let mut all_heights = vec![];
                    let mut after: Option<String> = None;
                    let mut before: Option<String> = None;

                    loop {
                        let variables = transfers::Variables {
                            after: after.clone(),
                            before: before.clone(),
                            first,
                            last,
                            sort_by: Some(sort_by.clone()),
                            block_height: None,
                            from_address: None,
                            to_address: None,
                            user_index: None,
                        };

                        let request_body = Transfers::build_query(variables);
                        let app = build_actix_app(httpd_context.clone());
                        let app = actix_web::test::init_service(app).await;

                        let request = actix_web::test::TestRequest::post()
                            .uri("/graphql")
                            .set_json(&request_body)
                            .to_request();

                        let response = actix_web::test::call_and_read_body(&app, request).await;
                        let response: Response<transfers::ResponseData> =
                            serde_json::from_slice(&response)?;

                        let data = response.data.unwrap();

                        match (first, last) {
                            (Some(_), None) => {
                                for node in data.transfers.nodes {
                                    all_heights.push(node.block_height);
                                }

                                if !data.transfers.page_info.has_next_page {
                                    break;
                                }
                                after = data.transfers.page_info.end_cursor;
                            },
                            (None, Some(_)) => {
                                for node in data.transfers.nodes.into_iter().rev() {
                                    all_heights.push(node.block_height);
                                }

                                if !data.transfers.page_info.has_previous_page {
                                    break;
                                }
                                before = data.transfers.page_info.start_cursor;
                            },
                            _ => break,
                        }
                    }

                    Ok(all_heights)
                }

                // 1. first with descending order
                let block_heights = paginate_all_transfers(
                    dango_httpd_context.clone(),
                    transfers::TransferSortBy::BLOCK_HEIGHT_DESC,
                    Some(transfers_count),
                    None,
                )
                .await?
                .into_iter()
                .map(|h| h as u64)
                .collect::<Vec<_>>();

                assert_that!(
                    block_heights
                        .clone()
                        .into_iter()
                        .unique()
                        .collect::<Vec<_>>()
                )
                .is_equal_to((1..=10).rev().collect::<Vec<_>>());

                // 2. first with ascending order
                let block_heights = paginate_all_transfers(
                    dango_httpd_context.clone(),
                    transfers::TransferSortBy::BLOCK_HEIGHT_ASC,
                    Some(transfers_count),
                    None,
                )
                .await?
                .into_iter()
                .map(|h| h as u64)
                .collect::<Vec<_>>();

                assert_that!(
                    block_heights
                        .clone()
                        .into_iter()
                        .unique()
                        .collect::<Vec<_>>()
                )
                .is_equal_to((1..=10).collect::<Vec<_>>());

                // 3. last with descending order
                let block_heights = paginate_all_transfers(
                    dango_httpd_context.clone(),
                    transfers::TransferSortBy::BLOCK_HEIGHT_DESC,
                    None,
                    Some(transfers_count),
                )
                .await?
                .into_iter()
                .map(|h| h as u64)
                .collect::<Vec<_>>();

                assert_that!(
                    block_heights
                        .clone()
                        .into_iter()
                        .unique()
                        .collect::<Vec<_>>()
                )
                .is_equal_to((1..=10).collect::<Vec<_>>());

                // 4. last with ascending order
                let block_heights = paginate_all_transfers(
                    dango_httpd_context.clone(),
                    transfers::TransferSortBy::BLOCK_HEIGHT_ASC,
                    None,
                    Some(transfers_count),
                )
                .await?
                .into_iter()
                .map(|h| h as u64)
                .collect::<Vec<_>>();

                assert_that!(
                    block_heights
                        .clone()
                        .into_iter()
                        .unique()
                        .collect::<Vec<_>>()
                )
                .is_equal_to((1..=10).rev().collect::<Vec<_>>());

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
        &account_factory::ExecuteMsg::RegisterAccount {
            params: AccountParams::Single(single::Params::new(accounts.user1.user_index())),
        },
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

    // Subscriptions still use raw GraphQL since indexer-client doesn't support them yet
    let graphql_query = r#"
      subscription Transfer {
        transfers {
          id
          idx
          createdAt
          blockHeight
          txHash
          fromAddress
          toAddress
          amount
          denom
        }
      }
    "#;

    let request_body = GraphQLCustomRequest {
        name: "transfers",
        query: graphql_query,
        variables: Default::default(),
    };

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
                let response =
                    parse_graphql_subscription_response::<Vec<entity::transfers::Model>>(
                        &mut framed,
                        name,
                    )
                    .await?;

                assert_that!(
                    response
                        .data
                        .into_iter()
                        .map(|t| t.block_height)
                        .collect::<Vec<_>>()
                )
                .is_equal_to(vec![1, 1]);

                crate_block_tx.send(2).await.unwrap();

                // 2nd response
                let response =
                    parse_graphql_subscription_response::<Vec<entity::transfers::Model>>(
                        &mut framed,
                        name,
                    )
                    .await?;

                assert_that!(
                    response
                        .data
                        .into_iter()
                        .map(|t| t.block_height)
                        .collect::<Vec<_>>()
                )
                .is_equal_to(vec![2]);

                crate_block_tx.send(3).await.unwrap();

                // 3rd response
                let response =
                    parse_graphql_subscription_response::<Vec<entity::transfers::Model>>(
                        &mut framed,
                        name,
                    )
                    .await?;

                assert_that!(
                    response
                        .data
                        .into_iter()
                        .map(|t| t.block_height)
                        .collect::<Vec<_>>()
                )
                .is_equal_to(vec![3]);

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
        &account_factory::ExecuteMsg::RegisterAccount {
            params: AccountParams::Single(single::Params::new(accounts.user1.user_index())),
        },
        Coins::one(usdc::DENOM.clone(), 100_000_000).unwrap(),
    )?];

    suite
        .send_messages_with_gas(
            &mut accounts.user1,
            50_000_000,
            NonEmpty::new_unchecked(msgs),
        )
        .should_succeed();

    // Subscriptions still use raw GraphQL since indexer-client doesn't support them yet
    let graphql_query = r#"
      subscription Transfer($address: String) {
        transfers(address: $address) {
          id
          idx
          createdAt
          blockHeight
          txHash
          fromAddress
          toAddress
          amount
          denom
        }
      }
    "#;

    let request_body = GraphQLCustomRequest {
        name: "transfers",
        query: graphql_query,
        variables: serde_json::json!({"address": accounts.user1.address})
            .as_object()
            .unwrap()
            .to_owned(),
    };

    let local_set = tokio::task::LocalSet::new();

    // Can't call this from LocalSet so using channels instead.
    let (create_block_tx, mut rx) = mpsc::channel::<u32>(1);
    tokio::spawn(async move {
        while let Some(_idx) = rx.recv().await {
            // Copied from benchmarks.rs
            let msgs = vec![Message::execute(
                contracts.account_factory,
                &account_factory::ExecuteMsg::RegisterAccount {
                    params: AccountParams::Single(single::Params::new(accounts.user1.user_index())),
                },
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
                let response =
                    parse_graphql_subscription_response::<Vec<entity::transfers::Model>>(
                        &mut framed,
                        name,
                    )
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
                let response =
                    parse_graphql_subscription_response::<Vec<entity::transfers::Model>>(
                        &mut framed,
                        name,
                    )
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
                let response =
                    parse_graphql_subscription_response::<Vec<entity::transfers::Model>>(
                        &mut framed,
                        name,
                    )
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
