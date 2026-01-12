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
                dango_mock_httpd::{BlockCreation, TestOption, get_mock_socket_addr, wait_for_server_ready},
                dango_testing::Preset,
                graphql_client::{GraphQLQuery, Response},
                serde_json::json,
            };

            $($(
                paste::paste! {
                    #[tokio::test]
                    async fn [<test_ $name:snake>]() -> anyhow::Result<()> {
                        let port = get_mock_socket_addr();

                        // Spawn server in separate thread with its own runtime
                        let _server_handle = std::thread::spawn(move || {
                            let rt = tokio::runtime::Builder::new_multi_thread()
                                .worker_threads(2)
                                .enable_all()
                                .build()
                                .unwrap();
                            rt.block_on(async {
                                #[cfg(feature = "tracing")]
                                tracing::info!("Starting mock HTTP server on port {port}");

                                if let Err(_error) = dango_mock_httpd::run(
                                    port,
                                    BlockCreation::OnBroadcast,
                                    None,
                                    TestOption::default(),
                                    GenesisOption::preset_test(),
                                    None,
                                )
                                .await
                                {
                                    #[cfg(feature = "tracing")]
                                    tracing::error!("Error running mock HTTP server: {_error}");
                                }
                            });
                        });

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
    ($({name: $name:ident, path: $path:literal $(, test_with: $var:expr)?}), * $(,)? ) => {
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
                dango_genesis::GenesisOption,
                dango_mock_httpd::{BlockCreation, TestOption, get_mock_socket_addr, wait_for_server_ready},
                dango_testing::Preset,
                futures::StreamExt,
                serde_json::json,
                std::time::Duration,
            };

            $($(
                paste::paste! {
                    #[tokio::test]
                    async fn [<test_ $name:snake>]() -> anyhow::Result<()> {
                        let port = get_mock_socket_addr();

                        // Spawn server in separate thread with its own runtime
                        let _server_handle = std::thread::spawn(move || {
                            let rt = tokio::runtime::Builder::new_multi_thread()
                                .worker_threads(2)
                                .enable_all()
                                .build()
                                .unwrap();
                            rt.block_on(async {
                                #[cfg(feature = "tracing")]
                                tracing::info!("Starting mock HTTP server on port {port}");

                                if let Err(_error) = dango_mock_httpd::run(
                                    port,
                                    BlockCreation::OnBroadcast,
                                    None,
                                    TestOption::default(),
                                    GenesisOption::preset_test(),
                                    None,
                                )
                                .await
                                {
                                    #[cfg(feature = "tracing")]
                                    tracing::error!("Error running mock HTTP server: {_error}");
                                }
                            });
                        });

                        wait_for_server_ready(port).await?;

                        let ws_url = format!("ws://localhost:{port}/graphql");
                        let client = crate::WsClient::new(&ws_url)?;

                        let mut stream = client.subscribe::<$name>($var).await?;

                        // For subscriptions, we just verify we can connect and start receiving
                        // We use a timeout since subscriptions are long-running
                        let result = tokio::time::timeout(
                            Duration::from_secs(5),
                            stream.next()
                        ).await;

                        // It's ok if we timeout (no data yet) or receive data
                        // The important thing is that the subscription was established
                        match result {
                            Ok(Some(Ok(_response))) => {
                                #[cfg(feature = "tracing")]
                                tracing::info!("Subscription response: {_response:#?}");
                                // Response received successfully
                            },
                            Ok(Some(Err(e))) => {
                                // Subscription error - this is a test failure
                                panic!("Subscription error: {e}");
                            },
                            Ok(None) => {
                                // Stream ended - unusual but not necessarily an error
                                #[cfg(feature = "tracing")]
                                tracing::info!("Subscription stream ended");
                            },
                            Err(_) => {
                                // Timeout - expected for subscriptions that don't immediately emit
                                #[cfg(feature = "tracing")]
                                tracing::info!("Subscription timeout (expected for some subscriptions)");
                            },
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
        test_with: crate::subscribe_block::Variables
    },
    {
        name: SubscribeAccounts,
        path: "src/schemas/subscriptions/accounts.graphql",
        test_with: crate::subscribe_accounts::Variables::default()
    },
    {
        name: SubscribeTransfers,
        path: "src/schemas/subscriptions/transfers.graphql",
        test_with: crate::subscribe_transfers::Variables::default()
    },
    {
        name: SubscribeTransactions,
        path: "src/schemas/subscriptions/transactions.graphql",
        test_with: crate::subscribe_transactions::Variables::default()
    },
    {
        name: SubscribeMessages,
        path: "src/schemas/subscriptions/messages.graphql",
        test_with: crate::subscribe_messages::Variables::default()
    },
    {
        name: SubscribeEvents,
        path: "src/schemas/subscriptions/events.graphql",
        test_with: crate::subscribe_events::Variables::default()
    },
    {
        name: SubscribeEventByAddresses,
        path: "src/schemas/subscriptions/eventByAddresses.graphql",
        test_with: crate::subscribe_event_by_addresses::Variables {
            addresses: vec!["0x0000000000000000000000000000000000000000".to_string()],
            since_block_height: None,
        }
    },
    {
        name: SubscribeCandles,
        path: "src/schemas/subscriptions/candles.graphql",
        test_with: crate::subscribe_candles::Variables {
            base_denom: "dango".to_string(),
            quote_denom: "bridge/usdc".to_string(),
            interval: crate::subscribe_candles::CandleInterval::ONE_MINUTE,
        }
    },
    {
        name: SubscribeTrades,
        path: "src/schemas/subscriptions/trades.graphql",
        test_with: crate::subscribe_trades::Variables {
            base_denom: "dango".to_string(),
            quote_denom: "bridge/usdc".to_string(),
        }
    },
    {
        name: SubscribeQueryApp,
        path: "src/schemas/subscriptions/queryApp.graphql",
        test_with: crate::subscribe_query_app::Variables {
            request: json!({"config":{}}),
            block_interval: 10,
        }
    },
    {
        name: SubscribeQueryStore,
        path: "src/schemas/subscriptions/queryStore.graphql",
        test_with: crate::subscribe_query_store::Variables {
            key: "Y2hhaW5faWQ=".to_string(),
            prove: false,
            block_interval: 10,
        }
    },
    {
        name: SubscribeQueryStatus,
        path: "src/schemas/subscriptions/queryStatus.graphql",
        test_with: crate::subscribe_query_status::Variables {
            block_interval: 10,
        }
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
