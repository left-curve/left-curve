use {
    super::build_actix_app,
    assert_json_diff::*,
    dango_testing::{HyperlaneTestSuite, create_user_and_account, setup_test_with_indexer},
    indexer_testing::{GraphQLCustomRequest, PaginatedResponse, call_graphql},
};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn query_user() -> anyhow::Result<()> {
    let (suite, mut accounts, codes, contracts, validator_sets, httpd_context) =
        setup_test_with_indexer();
    let mut suite = HyperlaneTestSuite::new(suite, validator_sets, &contracts);

    let user = create_user_and_account(&mut suite, &mut accounts, &contracts, &codes, "user");

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
                    "username": user.username.to_string(),
                    "publicKeys": [
                        {
                            "publicKey": user.first_key().to_string(),
                            "keyHash": user.first_key_hash().to_string(),
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
