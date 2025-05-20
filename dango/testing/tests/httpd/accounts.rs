use {
    super::build_actix_app,
    crate::accounts::account_factory::Account,
    assert_json_diff::*,
    dango_testing::{Factory, HyperlaneTestSuite, TestAccount, setup_test_with_indexer},
    dango_types::{
        account::single,
        account_factory::{self, AccountParams, RegisterUserData, Username},
        bank,
        constants::usdc,
    },
    grug::{
        Addressable, Coins, HashExt, QuerierExt, ResultExt, Uint128, btree_map, coins,
        setup_tracing_subscriber,
    },
    hyperlane_types::constants::solana,
    indexer_testing::{GraphQLCustomRequest, PaginatedResponse, call_graphql},
    std::str::FromStr,
};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn index_account_creations() -> anyhow::Result<()> {
    setup_tracing_subscriber(tracing::Level::INFO);

    let (suite, mut accounts, codes, contracts, validator_sets, httpd_context) =
        setup_test_with_indexer();
    let mut suite = HyperlaneTestSuite::new(suite, validator_sets, &contracts);

    let chain_id = suite.chain_id.clone();

    // Copied from `user_onboarding`` test
    // Create a new key offchain; then, predict what its address would be.
    let username = Username::from_str("user").unwrap();
    let user = TestAccount::new_random(username.clone()).predict_address(
        contracts.account_factory,
        0,
        codes.account_spot.to_bytes().hash256(),
        true,
    );

    // Make the initial deposit.
    suite
        .receive_warp_transfer(
            &mut accounts.owner,
            solana::DOMAIN,
            solana::USDC_WARP,
            &user,
            10_000_000,
        )
        .should_succeed();

    // The transfer should be an orphaned transfer. The bank contract should be
    // holding the 10 USDC.
    suite
        .query_balance(&contracts.bank, usdc::DENOM.clone())
        .should_succeed_and_equal(Uint128::new(10_000_000));

    // The orphaned transfer should have been recorded.
    suite
        .query_wasm_smart(contracts.bank, bank::QueryOrphanedTransferRequest {
            sender: contracts.gateway,
            recipient: user.address(),
        })
        .should_succeed_and_equal(coins! { usdc::DENOM.clone() => 10_000_000 });

    // User uses account factory as sender to send an empty transaction.
    // Account factory should interpret this action as the user wishes to create
    // an account and claim the funds held in IBC transfer contract.
    suite
        .execute(
            &mut Factory::new(contracts.account_factory),
            contracts.account_factory,
            &account_factory::ExecuteMsg::RegisterUser {
                seed: 0,
                username: user.username.clone(),
                key: user.first_key(),
                key_hash: user.first_key_hash(),
                signature: user
                    .sign_arbitrary(RegisterUserData {
                        username: user.username.clone(),
                        chain_id: chain_id.clone(),
                    })
                    .unwrap(),
            },
            Coins::new(),
        )
        .should_succeed();

    // The user's key should have been recorded in account factory.
    suite
        .query_wasm_smart(
            contracts.account_factory,
            account_factory::QueryKeysByUserRequest {
                username: user.username.clone(),
            },
        )
        .should_succeed_and_equal(btree_map! { user.first_key_hash() => user.first_key() });

    // The user's account info should have been recorded in account factory.
    // Note: a user's first ever account is always a spot account.
    suite
        .query_wasm_smart(
            contracts.account_factory,
            account_factory::QueryAccountsByUserRequest {
                username: user.username.clone(),
            },
        )
        .should_succeed_and_equal(btree_map! {
            user.address() => Account {
                // We have 10 genesis accounts (owner + users 1-9), indexed from
                // zero, so this one should have the index of 10.
                index: 10,
                params: AccountParams::Spot(single::Params::new(user.username.clone() )),
            },
        });

    // User's account should have been created with the correct token balance.
    suite
        .query_balance(&user, usdc::DENOM.clone())
        .should_succeed_and_equal(Uint128::new(10_000_000));

    // Force the runtime to wait for the async indexer task to finish
    suite.app.indexer.wait_for_finish();

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
                            "username": user.username.to_string(),
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
