use {
    super::build_actix_app,
    assert_json_diff::*,
    dango_testing::create_accounts,
    grug::setup_tracing_subscriber,
    indexer_testing::{GraphQLCustomRequest, PaginatedResponse, call_graphql},
};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn index_account_creations() -> anyhow::Result<()> {
    setup_tracing_subscriber(tracing::Level::INFO);

    let (_, test_account, httpd_context) = create_accounts();

    let graphql_query = r#"
      query Accounts {
      accounts {
          nodes {
            address
            accountIndex
            accountType
            createdAt
            createdBlockHeight
            users { username }
          }
          edges { node { address accountIndex accountType createdAt createdBlockHeight users { username } }  cursor }
          pageInfo { hasPreviousPage hasNextPage startCursor endCursor }
        }
      }
    "#;

    let request_body = GraphQLCustomRequest {
        name: "accounts",
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
                    "accountType": "SPOT",
                    "users": [
                        {
                            "username": test_account.username.to_string(),
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
