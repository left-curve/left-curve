use {
    super::build_actix_app,
    assert_json_diff::*,
    dango_testing::create_accounts,
    indexer_testing::{GraphQLCustomRequest, PaginatedResponse, call_graphql},
};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn query_user() -> anyhow::Result<()> {
    let (_, test_account, httpd_context) = create_accounts();

    let graphql_query = r#"
      query Users {
      users {
          nodes {
            username
            publicKeys { publicKey keyHash }
          }
          edges { node { username publicKeys { publicKey keyHash } }  cursor }
          pageInfo { hasPreviousPage hasNextPage startCursor endCursor }
        }
      }
    "#;

    let request_body = GraphQLCustomRequest {
        name: "users",
        query: graphql_query,
        variables: Default::default(),
    };

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let app = build_actix_app(httpd_context);

                let response =
                    call_graphql::<PaginatedResponse<serde_json::Value>>(app, request_body).await?;

                let expected_data = serde_json::json!({
                    "username": test_account.username.to_string(),
                    "publicKeys": [
                        {
                            "publicKey": test_account.first_key().to_string(),
                            "keyHash": test_account.first_key_hash().to_string(),
                        }
                    ],
                });

                assert_json_include!(actual: response.data.edges[0].node, expected: expected_data);

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}
