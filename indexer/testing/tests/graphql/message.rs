use {
    assertor::*,
    graphql_client::GraphQLQuery,
    grug_types::{BroadcastClientExt, Coins, Denom, GasOption, Message, ResultExt},
    indexer_client::{Messages, SubscribeMessages, messages, subscribe_messages},
    indexer_testing::{
        GraphQLCustomRequest, PaginationDirection,
        block::{create_block, create_blocks},
        build_app_service, call_graphql_query, call_ws_graphql_stream, messages_query,
        paginate_messages, parse_graphql_subscription_response,
    },
    std::str::FromStr,
    tokio::sync::mpsc,
};

#[tokio::test(flavor = "multi_thread")]
async fn graphql_returns_messages() -> anyhow::Result<()> {
    let (httpd_context, _client, accounts) = create_block().await?;

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let app = build_app_service(httpd_context);
                let query_body = Messages::build_query(messages::Variables::default());

                let response =
                    call_graphql_query::<_, messages::ResponseData, _, _, _>(app, query_body)
                        .await?;

                assert_that!(response.data).is_some();
                let data = response.data.unwrap();

                assert_that!(data.messages.nodes).has_length(1);
                assert_that!(data.messages.nodes[0].sender_addr.as_str())
                    .is_equal_to(accounts["sender"].address.to_string().as_str());

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread")]
async fn graphql_paginate_messages() -> anyhow::Result<()> {
    let (httpd_context, _client, _) = create_blocks(10).await?;

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let page_size = 2;

                // 1. first with descending order
                let messages = paginate_messages(
                    httpd_context.clone(),
                    page_size,
                    messages_query::Variables {
                        sort_by: Some(messages_query::MessageSortBy::BLOCK_HEIGHT_DESC),
                        ..Default::default()
                    },
                    PaginationDirection::Forward,
                )
                .await?;
                let block_heights: Vec<_> = messages.iter().map(|m| m.block_height).collect();
                assert_that!(block_heights).is_equal_to((1i64..=10).rev().collect::<Vec<_>>());

                // 2. first with ascending order
                let messages = paginate_messages(
                    httpd_context.clone(),
                    page_size,
                    messages_query::Variables {
                        sort_by: Some(messages_query::MessageSortBy::BLOCK_HEIGHT_ASC),
                        ..Default::default()
                    },
                    PaginationDirection::Forward,
                )
                .await?;
                let block_heights: Vec<_> = messages.iter().map(|m| m.block_height).collect();
                assert_that!(block_heights).is_equal_to((1i64..=10).collect::<Vec<_>>());

                // 3. last with descending order
                let messages = paginate_messages(
                    httpd_context.clone(),
                    page_size,
                    messages_query::Variables {
                        sort_by: Some(messages_query::MessageSortBy::BLOCK_HEIGHT_DESC),
                        ..Default::default()
                    },
                    PaginationDirection::Backward,
                )
                .await?;
                let block_heights: Vec<_> = messages.iter().map(|m| m.block_height).collect();
                assert_that!(block_heights).is_equal_to((1i64..=10).collect::<Vec<_>>());

                // 4. last with ascending order
                let messages = paginate_messages(
                    httpd_context.clone(),
                    page_size,
                    messages_query::Variables {
                        sort_by: Some(messages_query::MessageSortBy::BLOCK_HEIGHT_ASC),
                        ..Default::default()
                    },
                    PaginationDirection::Backward,
                )
                .await?;
                let block_heights: Vec<_> = messages.iter().map(|m| m.block_height).collect();
                assert_that!(block_heights).is_equal_to((1i64..=10).rev().collect::<Vec<_>>());

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread")]
async fn graphql_subscribe_to_messages() -> anyhow::Result<()> {
    let (httpd_context, client, mut accounts) = create_block().await?;

    // Use typed subscription from indexer-client
    let request_body = GraphQLCustomRequest::from_query_body(
        SubscribeMessages::build_query(subscribe_messages::Variables::default()),
        "messages",
    );

    let (crate_block_tx, mut rx) = mpsc::channel::<u32>(1);

    let owner_addr = accounts["sender"].address.to_string();

    // Can't call this from LocalSet so using channels instead.
    tokio::spawn(async move {
        while rx.recv().await.is_some() {
            let to = accounts["owner"].address;

            let chain_id = client.chain_id().await;

            client
                .send_message(
                    &mut accounts["sender"],
                    Message::transfer(to, Coins::one(Denom::from_str("ugrug")?, 2_000)?)?,
                    GasOption::Predefined { gas_limit: 2000 },
                    &chain_id,
                )
                .await
                .should_succeed();
        }

        Ok::<(), anyhow::Error>(())
    });

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let name = request_body.name;
                let (_srv, _ws, mut framed) =
                    call_ws_graphql_stream(httpd_context, build_app_service, request_body).await?;

                // 1st response is always the existing last block
                let response = parse_graphql_subscription_response::<
                    Vec<subscribe_messages::SubscribeMessagesMessages>,
                >(&mut framed, name)
                .await?;

                assert_that!(response.data).has_length(1);
                let msg = response.data.first().unwrap();
                assert_that!(msg.block_height).is_equal_to(1);
                assert_that!(msg.method_name.as_str()).is_equal_to("transfer");
                assert_that!(msg.sender_addr.as_str()).is_equal_to(owner_addr.as_str());

                crate_block_tx.send(2).await?;

                // 2nd response
                let response = parse_graphql_subscription_response::<
                    Vec<subscribe_messages::SubscribeMessagesMessages>,
                >(&mut framed, name)
                .await?;

                assert_that!(response.data).has_length(1);
                let msg = response.data.first().unwrap();
                assert_that!(msg.block_height).is_equal_to(2);
                assert_that!(msg.method_name.as_str()).is_equal_to("transfer");
                assert_that!(msg.sender_addr.as_str()).is_equal_to(owner_addr.as_str());

                crate_block_tx.send(3).await?;

                // 3rd response
                let response = parse_graphql_subscription_response::<
                    Vec<subscribe_messages::SubscribeMessagesMessages>,
                >(&mut framed, name)
                .await?;

                assert_that!(response.data).has_length(1);
                let msg = response.data.first().unwrap();
                assert_that!(msg.block_height).is_equal_to(3);
                assert_that!(msg.method_name.as_str()).is_equal_to("transfer");
                assert_that!(msg.sender_addr.as_str()).is_equal_to(owner_addr.as_str());

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}
