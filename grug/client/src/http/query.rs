type DateTime = chrono::DateTime<chrono::Utc>;

#[cfg(test)]
const GRAPHQL_URL: &str = "https://devnet-graphql.dango.exchange";

pub trait Variables {
    type Query: graphql_client::GraphQLQuery<Variables = Self>;
}

macro_rules! generate_queries {
    ($({name: $name:ident, path: $path:literal, test_with: $var:expr}), *) => {
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
        name: GetBlock,
        path: "src/http/schemas/queries/block.graphql",
        test_with: crate::query::get_block::Variables { height: 1 }
    },
    {
        name: QueryApp,
        path: "src/http/schemas/queries/queryApp.graphql",
        test_with: crate::query::query_app::Variables {
            request: r#"{"config":{}}"#.to_string(),
            height: 1
        }
    }
}
