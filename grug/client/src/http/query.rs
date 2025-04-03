#[cfg(test)]
const GRAPHQL_URL: &str = "https://devnet-graphql.dango.exchange";

pub trait Variables {
    type Query: graphql_client::GraphQLQuery<Variables = Self>;
}

macro_rules! generate_queries {
    ($({name: $name:ident, path: $path:literal, test_with: $var:expr}), * $(,)? ) => {
        $(
            #[derive(graphql_client::GraphQLQuery)]
            #[graphql(
                schema_path = "src/http/schemas/schema.graphql",
                query_path = $path,
                response_derives = "Debug"
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
            use {super::*, graphql_client::{GraphQLQuery, Response}};

            $(
                paste::paste! {
                    #[tokio::test]
                    async fn [<test_ $name:snake>]() {
                        let client = reqwest::Client::builder().build().unwrap();
                        let query = $name::build_query($var);
                        let response = client.post(GRAPHQL_URL).json(&query).send().await.unwrap();
                        response
                            .json::<Response<[<$name:snake>]::ResponseData>>()
                            .await
                            .unwrap();
                    }
                }
            )*
        }
    };
}

generate_queries! {
    {
        name: QueryApp,
        path: "src/http/schemas/queries/queryApp.graphql",
        test_with: crate::query::query_app::Variables {
            request: r#"{"config":{}}"#.to_string(),
            height: None
        }
    },
    {
        name: QueryStore,
        path: "src/http/schemas/queries/queryStore.graphql",
        test_with: crate::query::query_store::Variables {
            key: "Y2hhaW5faWQ=".to_string(),
            height: None,
            prove: true
        }
    },
    {
        name: Simulate,
        path: "src/http/schemas/queries/Simulate.graphql",
        test_with: crate::query::simulate::Variables {
            tx: r#"{
                    "sender": "0x33361de42571d6aa20c37daa6da4b5ab67bfaad9",
                    "msgs": [
                        {
                        "transfer": {
                            "0x01bba610cbbfe9df0c99b8862f3ad41b2f646553": {
                            "hyp/all/btc": "100"
                            }
                        }
                        }
                    ],
                    "data": {
                        "chain_id": "dev-6",
                        "nonce": 1,
                        "username": "owner"
                    }
                }"#.to_string(),
        }
    }
}
