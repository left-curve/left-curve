use {
    assertor::*,
    graphql_client::GraphQLQuery,
    grug_types::{BroadcastClientExt, Coins, Denom, GasOption, Message, ResultExt},
    indexer_client::{Messages, messages},
    indexer_testing::{
        GraphQLCustomRequest,
        block::{create_block, create_blocks},
        build_app_service, call_graphql_query, call_ws_graphql_stream,
        parse_graphql_subscription_response,
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
                let messages_count = 2;

                // Helper to paginate through all messages
                async fn paginate_all_messages(
                    httpd_context: indexer_httpd::context::Context,
                    sort_by: messages::MessageSortBy,
                    first: Option<i64>,
                    last: Option<i64>,
                ) -> anyhow::Result<Vec<i64>> {
                    let mut all_heights = vec![];
                    let mut after: Option<String> = None;
                    let mut before: Option<String> = None;

                    loop {
                        let variables = messages::Variables {
                            after: after.clone(),
                            before: before.clone(),
                            first,
                            last,
                            sort_by: Some(sort_by.clone()),
                            ..Default::default()
                        };

                        let app = build_app_service(httpd_context.clone());
                        let query_body = Messages::build_query(variables);
                        let response = call_graphql_query::<_, messages::ResponseData, _, _, _>(
                            app, query_body,
                        )
                        .await?;

                        let data = response.data.unwrap();

                        match (first, last) {
                            (Some(_), None) => {
                                for node in data.messages.nodes {
                                    all_heights.push(node.block_height);
                                }

                                if !data.messages.page_info.has_next_page {
                                    break;
                                }
                                after = data.messages.page_info.end_cursor;
                            },
                            (None, Some(_)) => {
                                for node in data.messages.nodes.into_iter().rev() {
                                    all_heights.push(node.block_height);
                                }

                                if !data.messages.page_info.has_previous_page {
                                    break;
                                }
                                before = data.messages.page_info.start_cursor;
                            },
                            _ => break,
                        }
                    }

                    Ok(all_heights)
                }

                // 1. first with descending order
                let block_heights = paginate_all_messages(
                    httpd_context.clone(),
                    messages::MessageSortBy::BLOCK_HEIGHT_DESC,
                    Some(messages_count),
                    None,
                )
                .await?;

                assert_that!(block_heights).is_equal_to((1i64..=10).rev().collect::<Vec<_>>());

                // 2. first with ascending order
                let block_heights = paginate_all_messages(
                    httpd_context.clone(),
                    messages::MessageSortBy::BLOCK_HEIGHT_ASC,
                    Some(messages_count),
                    None,
                )
                .await?;

                assert_that!(block_heights).is_equal_to((1i64..=10).collect::<Vec<_>>());

                // 3. last with descending order
                let block_heights = paginate_all_messages(
                    httpd_context.clone(),
                    messages::MessageSortBy::BLOCK_HEIGHT_DESC,
                    None,
                    Some(messages_count),
                )
                .await?;

                assert_that!(block_heights).is_equal_to((1i64..=10).collect::<Vec<_>>());

                // 4. last with ascending order
                let block_heights = paginate_all_messages(
                    httpd_context.clone(),
                    messages::MessageSortBy::BLOCK_HEIGHT_ASC,
                    None,
                    Some(messages_count),
                )
                .await?;

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

    // Subscriptions still use raw GraphQL since indexer-client doesn't support them yet
    let graphql_query = r#"
      subscription Messages {
        messages {
          data
          blockHeight
          createdAt
          orderIdx
          methodName
          contractAddr
          senderAddr
        }
      }
    "#;

    let request_body = GraphQLCustomRequest {
        name: "messages",
        query: graphql_query,
        variables: Default::default(),
    };

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

                // Define a simple struct to capture subscription response
                #[derive(serde::Deserialize, Debug)]
                #[serde(rename_all = "camelCase")]
                struct SubscriptionMessage {
                    block_height: i64,
                    method_name: String,
                    sender_addr: String,
                }

                // 1st response is always the existing last block
                let response = parse_graphql_subscription_response::<Vec<SubscriptionMessage>>(
                    &mut framed,
                    name,
                )
                .await?;

                assert_that!(response.data.first().unwrap().block_height).is_equal_to(1);
                assert_that!(response.data.first().unwrap().method_name.as_str())
                    .is_equal_to("transfer");
                assert_that!(response.data.first().unwrap().sender_addr.as_str())
                    .is_equal_to(owner_addr.as_str());
                assert_that!(response.data).has_length(1);

                crate_block_tx.send(2).await?;

                // 2nd response
                let response = parse_graphql_subscription_response::<Vec<SubscriptionMessage>>(
                    &mut framed,
                    name,
                )
                .await?;

                assert_that!(response.data.first().unwrap().block_height).is_equal_to(2);
                assert_that!(response.data.first().unwrap().method_name.as_str())
                    .is_equal_to("transfer");
                assert_that!(response.data.first().unwrap().sender_addr.as_str())
                    .is_equal_to(owner_addr.as_str());
                assert_that!(response.data).has_length(1);

                crate_block_tx.send(3).await?;

                // 3rd response
                let response = parse_graphql_subscription_response::<Vec<SubscriptionMessage>>(
                    &mut framed,
                    name,
                )
                .await?;

                assert_that!(response.data.first().unwrap().block_height).is_equal_to(3);
                assert_that!(response.data.first().unwrap().method_name.as_str())
                    .is_equal_to("transfer");
                assert_that!(response.data.first().unwrap().sender_addr.as_str())
                    .is_equal_to(owner_addr.as_str());
                assert_that!(response.data).has_length(1);

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}
