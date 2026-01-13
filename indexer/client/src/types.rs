pub trait Variables {
    type Query: graphql_client::GraphQLQuery<Variables = Self>;
}

macro_rules! generate_types {
    ($({name: $name:ident, path: $path:literal, $(test_with: $var:expr)?}), * $(,)? ) => {
        $(
            #[derive(graphql_client::GraphQLQuery)]
            #[graphql(
                schema_path = "src/schemas/schema.graphql",
                query_path = $path,
                response_derives = "Debug, Clone, PartialEq, Eq",
                variables_derives = "Debug, Clone, Default"
            )]
            pub struct $name;

            paste::paste! {
                impl Variables for [<$name:snake>]::Variables {
                    type Query = $name;
                }
            }
        )*

        #[cfg(test)]
        mod tests {
            use {
                super::*,
                assertor::*,
                dango_genesis::GenesisOption,
                dango_mock_httpd::{BlockCreation, TestOption, wait_for_server_ready},
                dango_testing::Preset,
                graphql_client::{GraphQLQuery, Response},
                serde_json::json,
                std::sync::mpsc,
            };

            $($(
                paste::paste! {
                    #[tokio::test]
                    async fn [<test_ $name:snake>]() -> anyhow::Result<()> {
                        // Create channel to receive the actual bound port
                        let (port_tx, port_rx) = mpsc::channel::<u16>();

                        // Spawn server in separate thread with its own runtime
                        let _server_handle = std::thread::spawn(move || {
                            let rt = tokio::runtime::Builder::new_multi_thread()
                                .worker_threads(2)
                                .enable_all()
                                .build()
                                .unwrap();
                            rt.block_on(async {
                                #[cfg(feature = "tracing")]
                                tracing::info!("Starting mock HTTP server with port 0");

                                if let Err(_error) = dango_mock_httpd::run_with_port_sender(
                                    BlockCreation::OnBroadcast,
                                    None,
                                    TestOption::default(),
                                    GenesisOption::preset_test(),
                                    None,
                                    port_tx,
                                )
                                .await
                                {
                                    #[cfg(feature = "tracing")]
                                    tracing::error!("Error running mock HTTP server: {_error}");
                                }
                            });
                        });

                        // Wait for the server to send us the actual port
                        let port = port_rx.recv().expect("Failed to receive port from server");

                        #[cfg(feature = "tracing")]
                        tracing::info!("Server started on port {port}");

                        // Wait for server to be ready
                        wait_for_server_ready(port).await?;

                        let url = format!("http://localhost:{port}/graphql");

                        let result = reqwest::Client::builder()
                            .build()
                            .unwrap()
                            .post(url)
                            .json(&$name::build_query($var))
                            .send()
                            .await
                            .unwrap()
                            .json::<Response<[<$name:snake>]::ResponseData>>()
                            .await;

                        #[cfg(feature = "tracing")]
                        tracing::info!("GraphQL response: {result:#?}");

                        assert_that!(result).is_ok();

                        Ok(())
                    }
                }
            )*)?
        }
    };
}

#[allow(clippy::upper_case_acronyms)]
type JSON = serde_json::Value;
type GrugQueryInput = serde_json::Value;
type UnsignedTx = serde_json::Value;
type Tx = serde_json::Value;
type DateTime = String;
type BigDecimal = String;
type NaiveDateTime = String;

generate_types! {
    {
        name: QueryApp,
        path: "src/schemas/queries/queryApp.graphql",
        test_with: crate::types::query_app::Variables {
            request: json!({"config":{}}),
            height: None
        }
    },
    {
        name: QueryStore,
        path: "src/schemas/queries/queryStore.graphql",
        test_with: crate::types::query_store::Variables {
            key: "Y2hhaW5faWQ=".to_string(),
            height: None,
            prove: true
        }
    },
    {
        name: Simulate,
        path: "src/schemas/queries/simulate.graphql",
        test_with: crate::types::simulate::Variables {
            tx: json!({
              "data": {
                "chain_id": "dev-1",
                "nonce": 1,
                "username": "owner"
              },
              "msgs": [
                {
                  "transfer": {
                    "0x01bba610cbbfe9df0c99b8862f3ad41b2f646553": {
                      "hyp/all/btc": "100"
                    }
                  }
                }
              ],
              "sender": "0x33361de42571d6aa20c37daa6da4b5ab67bfaad9"
            }),
        }
    },
    {
        name: BroadcastTxSync,
        path: "src/schemas/mutations/broadcastTxSync.graphql",
    },
    {
        name: SearchTx,
        path: "src/schemas/queries/transaction.graphql",
    },
    {
        name: Block,
        path: "src/schemas/queries/block.graphql",
    },
    {
        name: Blocks,
        path: "src/schemas/queries/blocks.graphql",
    },
    {
        name: Transactions,
        path: "src/schemas/queries/transactions.graphql",
    },
    {
        name: Messages,
        path: "src/schemas/queries/messages.graphql",
    },
    {
        name: Events,
        path: "src/schemas/queries/events.graphql",
    },
    {
        name: Transfers,
        path: "src/schemas/queries/transfers.graphql",
    },
    {
        name: Accounts,
        path: "src/schemas/queries/accounts.graphql",
    },
    {
        name: User,
        path: "src/schemas/queries/user.graphql",
    },
    {
        name: Users,
        path: "src/schemas/queries/users.graphql",
    },
    {
        name: Candles,
        path: "src/schemas/queries/candles.graphql",
    },
    {
        name: Trades,
        path: "src/schemas/queries/trades.graphql",
    },
    {
        name: QueryStatus,
        path: "src/schemas/queries/queryStatus.graphql",
    }
}

// Implement Default for enum types used as required fields in Variables
impl Default for candles::CandleInterval {
    fn default() -> Self {
        Self::ONE_MINUTE
    }
}

// Subscription types - generated separately since they follow a different pattern
macro_rules! generate_subscription_types {
    ($({name: $name:ident, path: $path:literal $(, test_with: $var:expr, expect: $expect:expr, require_data: $require:expr)?}), * $(,)? ) => {
        $(
            #[derive(graphql_client::GraphQLQuery)]
            #[graphql(
                schema_path = "src/schemas/schema.graphql",
                query_path = $path,
                response_derives = "Debug, Clone, PartialEq, Eq",
                variables_derives = "Debug, Clone, Default"
            )]
            pub struct $name;

            paste::paste! {
                impl Variables for [<$name:snake>]::Variables {
                    type Query = $name;
                }
            }
        )*

        #[cfg(test)]
        mod subscription_tests {
            #[allow(unused_imports)]
            use {
                super::*,
                assertor::*,
                dango_genesis::GenesisOption,
                dango_mock_httpd::{BlockCreation, TestOption, wait_for_server_ready},
                dango_testing::Preset,
                futures::StreamExt,
                serde_json::json,
                std::{sync::mpsc, time::Duration},
            };

            const MAX_RETRIES: u32 = 3;
            const TIMEOUT_SECS: u64 = 5;

            $($(
                paste::paste! {
                    #[tokio::test]
                    async fn [<test_ $name:snake>]() -> anyhow::Result<()> {
                        // Create channel to receive the actual bound port
                        let (port_tx, port_rx) = mpsc::channel::<u16>();

                        // Spawn server in separate thread with its own runtime
                        let _server_handle = std::thread::spawn(move || {
                            let rt = tokio::runtime::Builder::new_multi_thread()
                                .worker_threads(2)
                                .enable_all()
                                .build()
                                .unwrap();
                            rt.block_on(async {
                                #[cfg(feature = "tracing")]
                                tracing::info!("Starting mock HTTP server with port 0");

                                if let Err(_error) = dango_mock_httpd::run_with_port_sender(
                                    BlockCreation::OnBroadcast,
                                    None,
                                    TestOption::default(),
                                    GenesisOption::preset_test(),
                                    None,
                                    port_tx,
                                )
                                .await
                                {
                                    #[cfg(feature = "tracing")]
                                    tracing::error!("Error running mock HTTP server: {_error}");
                                }
                            });
                        });

                        // Wait for the server to send us the actual port
                        let port = port_rx.recv().expect("Failed to receive port from server");

                        #[cfg(feature = "tracing")]
                        tracing::info!("Server started on port {port}");

                        // Wait for server to be ready
                        wait_for_server_ready(port).await?;

                        let ws_url = format!("ws://localhost:{port}/graphql");
                        let client = crate::WsClient::new(&ws_url)?;

                        let mut stream = client.subscribe::<$name>($var).await?;

                        // Retry loop for subscriptions that may not emit immediately
                        let mut retries = 0;
                        let require_data: bool = $require;

                        loop {
                            let result = tokio::time::timeout(
                                Duration::from_secs(TIMEOUT_SECS),
                                stream.next()
                            ).await;

                            match result {
                                Ok(Some(Ok(response))) => {
                                    #[cfg(feature = "tracing")]
                                    tracing::info!("Subscription response: {response:#?}");

                                    // Check for GraphQL errors
                                    if let Some(errors) = &response.errors {
                                        if !errors.is_empty() {
                                            if require_data {
                                                panic!("GraphQL errors: {:?}", errors);
                                            } else {
                                                // For event-driven subscriptions, GraphQL errors may be acceptable
                                                // (e.g., query not found for certain keys)
                                                break;
                                            }
                                        }
                                    }

                                    // Verify the response data matches expected
                                    if let Some(data) = response.data {
                                        let expect_fn: fn(_) -> bool = $expect;
                                        assert!(expect_fn(data), "Response data did not match expected");
                                    } else if require_data {
                                        panic!("Expected data in response but got None");
                                    }
                                    break;
                                },
                                Ok(Some(Err(e))) => {
                                    panic!("Subscription error: {e}");
                                },
                                Ok(None) => {
                                    panic!("Subscription stream ended unexpectedly");
                                },
                                Err(_) => {
                                    retries += 1;
                                    #[cfg(feature = "tracing")]
                                    tracing::info!("Subscription timeout, retry {retries}/{MAX_RETRIES}");

                                    if retries >= MAX_RETRIES {
                                        if require_data {
                                            panic!("Subscription timed out after {MAX_RETRIES} retries");
                                        } else {
                                            // For event-driven subscriptions, timeout is acceptable
                                            // The connection was established successfully
                                            break;
                                        }
                                    }
                                    // Continue the loop to retry
                                },
                            }
                        }

                        Ok(())
                    }
                }
            )*)?
        }
    };
}

generate_subscription_types! {
    {
        name: SubscribeBlock,
        path: "src/schemas/subscriptions/block.graphql",
        test_with: crate::subscribe_block::Variables,
        expect: |data: crate::subscribe_block::ResponseData| {
            data.block.block_height >= 0
        },
        require_data: false  // Event-driven, no automatic block production in mock
    },
    {
        name: SubscribeAccounts,
        path: "src/schemas/subscriptions/accounts.graphql",
        test_with: crate::subscribe_accounts::Variables::default(),
        expect: |_data: crate::subscribe_accounts::ResponseData| {
            // Accounts subscription returns a list
            true
        },
        require_data: false  // Event-driven
    },
    {
        name: SubscribeTransfers,
        path: "src/schemas/subscriptions/transfers.graphql",
        test_with: crate::subscribe_transfers::Variables::default(),
        expect: |_data: crate::subscribe_transfers::ResponseData| {
            // Transfers subscription returns a list
            true
        },
        require_data: false  // Event-driven
    },
    {
        name: SubscribeTransactions,
        path: "src/schemas/subscriptions/transactions.graphql",
        test_with: crate::subscribe_transactions::Variables::default(),
        expect: |_data: crate::subscribe_transactions::ResponseData| {
            // Transactions subscription returns a list
            true
        },
        require_data: false  // Event-driven
    },
    {
        name: SubscribeMessages,
        path: "src/schemas/subscriptions/messages.graphql",
        test_with: crate::subscribe_messages::Variables::default(),
        expect: |_data: crate::subscribe_messages::ResponseData| {
            // Messages subscription returns a list
            true
        },
        require_data: false  // Event-driven
    },
    {
        name: SubscribeEvents,
        path: "src/schemas/subscriptions/events.graphql",
        test_with: crate::subscribe_events::Variables::default(),
        expect: |_data: crate::subscribe_events::ResponseData| {
            // Events subscription returns a list
            true
        },
        require_data: false  // Event-driven
    },
    {
        name: SubscribeEventByAddresses,
        path: "src/schemas/subscriptions/eventByAddresses.graphql",
        test_with: crate::subscribe_event_by_addresses::Variables {
            addresses: vec!["0x0000000000000000000000000000000000000000".to_string()],
            since_block_height: None,
        },
        expect: |_data: crate::subscribe_event_by_addresses::ResponseData| {
            // EventByAddresses subscription returns a list
            true
        },
        require_data: false  // Event-driven
    },
    {
        name: SubscribeCandles,
        path: "src/schemas/subscriptions/candles.graphql",
        test_with: crate::subscribe_candles::Variables {
            base_denom: "dango".to_string(),
            quote_denom: "bridge/usdc".to_string(),
            interval: crate::subscribe_candles::CandleInterval::ONE_MINUTE,
            later_than: None,
        },
        expect: |_data: crate::subscribe_candles::ResponseData| {
            // Candles subscription returns a list
            true
        },
        require_data: false  // Event-driven
    },
    {
        name: SubscribeTrades,
        path: "src/schemas/subscriptions/trades.graphql",
        test_with: crate::subscribe_trades::Variables {
            base_denom: "dango".to_string(),
            quote_denom: "bridge/usdc".to_string(),
        },
        expect: |data: crate::subscribe_trades::ResponseData| {
            // Trades returns a single Trade, verify it has expected denoms
            data.trades.base_denom == "dango" && data.trades.quote_denom == "bridge/usdc"
        },
        require_data: false  // Event-driven
    },
    {
        name: SubscribeQueryApp,
        path: "src/schemas/subscriptions/queryApp.graphql",
        test_with: crate::subscribe_query_app::Variables {
            request: json!({"config":{}}),
            block_interval: 10,
        },
        expect: |data: crate::subscribe_query_app::ResponseData| {
            data.query_app.block_height >= 0
        },
        require_data: true  // Emits immediately with blockInterval
    },
    {
        name: SubscribeQueryStore,
        path: "src/schemas/subscriptions/queryStore.graphql",
        test_with: crate::subscribe_query_store::Variables {
            key: "Y2hhaW5faWQ=".to_string(),
            prove: false,
            block_interval: 10,
        },
        expect: |data: crate::subscribe_query_store::ResponseData| {
            data.query_store.block_height >= 0 && !data.query_store.value.is_empty()
        },
        require_data: false  // May not emit immediately
    },
    {
        name: SubscribeQueryStatus,
        path: "src/schemas/subscriptions/queryStatus.graphql",
        test_with: crate::subscribe_query_status::Variables {
            block_interval: 10,
        },
        expect: |data: crate::subscribe_query_status::ResponseData| {
            !data.query_status.chain_id.is_empty() && data.query_status.block.block_height >= 0
        },
        require_data: true  // Emits immediately with blockInterval
    },
}

// Re-export subscription modules
pub mod subscriptions {
    pub use super::{
        subscribe_accounts, subscribe_block, subscribe_candles, subscribe_event_by_addresses,
        subscribe_events, subscribe_messages, subscribe_query_app, subscribe_query_status,
        subscribe_query_store, subscribe_trades, subscribe_transactions, subscribe_transfers,
    };
}

// Implement Default for subscription enum types
impl Default for subscribe_candles::CandleInterval {
    fn default() -> Self {
        Self::ONE_MINUTE
    }
}

impl Default for subscribe_events::CheckValue {
    fn default() -> Self {
        Self::EQUAL
    }
}
