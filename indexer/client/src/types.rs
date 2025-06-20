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
                response_derives = "Debug",
                variables_derives = "Debug"
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
            // #[cfg(feature = "tracing")]
            // use grug::setup_tracing_subscriber;

            $($(
                paste::paste! {
                    #[tokio::test]
                    async fn [<test_ $name:snake>]() -> anyhow::Result<()> {
                        // #[cfg(feature = "tracing")]
                        // setup_tracing_subscriber(tracing::Level::WARN);

                        let port = get_mock_socket_addr();

                        // Spawn server in separate thread with its own runtime
                        let _server_handle = std::thread::spawn(move || {
                            let rt = tokio::runtime::Runtime::new().unwrap();
                            rt.block_on(async {
                                #[cfg(feature = "tracing")]
                                tracing::info!("Starting mock HTTP server on port {port}");

                                if let Err(_error) = dango_mock_httpd::run(
                                    port,
                                    BlockCreation::OnBroadcast,
                                    None,
                                    TestOption::default(),
                                    GenesisOption::preset_test(),
                                    true,
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
    }
}
