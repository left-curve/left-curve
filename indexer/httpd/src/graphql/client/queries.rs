use graphql_client::GraphQLQuery;

type DateTime = chrono::DateTime<chrono::Utc>;

macro_rules! query {
    ($name:ident, $path:literal, {$($variables:ident: $value:expr),*}) => {
        #[derive(GraphQLQuery)]
        #[graphql(
            schema_path = "src/graphql/schemas/schema.graphql",
            query_path = $path,
            response_derives = "Debug"
        )]
        pub struct $name;

        paste::paste! {
            #[cfg(test)]
            mod [<$name:snake _tests>] {
                use {
                    super::{[<$name:snake>]::{Variables, ResponseData}, $name},
                    graphql_client::{GraphQLQuery, Response},
                };

                #[tokio::test]
                async fn [<test_ $name:snake>]() {
                    let url = "https://devnet-graphql.dango.exchange";
                    let client = reqwest::Client::builder().build().unwrap();
                    let query = $name::build_query(Variables { $($variables: $value),* });
                    let response = client.post(url).json(&query).send().await.unwrap();
                    response
                        .json::<Response<ResponseData>>()
                        .await
                        .unwrap();
                }

            }
        }
    };
}

query!(
    GetBlock,
    "src/graphql/schemas/queries/block.graphql",
    {
        height: 1
    }
);

query!(
    QueryApp,
    "src/graphql/schemas/queries/queryApp.graphql",
    {
        request: r#"{"config":{}}"#.to_string(),
        height: 1
    }
);
