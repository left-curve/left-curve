#[cfg(test)]
const GRAPHQL_URL: &str = "https://devnet-graphql.dango.exchange";

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
                graphql_client::{GraphQLQuery, Response},
                serde_json::json,
            };

            $($(
                paste::paste! {
                    #[tokio::test]
                    #[ignore = "this test requires interactions with an external server"]
                    async fn [<test_ $name:snake>]() {
                        let result = reqwest::Client::builder()
                            .build()
                            .unwrap()
                            .post(GRAPHQL_URL)
                            .json(&$name::build_query($var))
                            .send()
                            .await
                            .unwrap()
                            .json::<Response<[<$name:snake>]::ResponseData>>()
                            .await;
                        assert!(result.is_ok());
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
                "chain_id": "dev-6",
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
    }
}
