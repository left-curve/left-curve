use {
    assertor::*,
    dango_gateway::REVERSE_ROUTES,
    dango_testing::{HyperlaneTestSuite, setup_test, setup_test_with_indexer},
    dango_types::{
        constants::{sol, usdc},
        gateway::{self, Remote},
        warp::TokenMessage,
    },
    grug::{
        Addr, Addressable, BalanceChange, HashExt, NumberConst, QuerierExt, ResultExt, StdError,
        Uint128, btree_map, coins,
    },
    grug_app::Indexer,
    hyperlane_testing::constants::MOCK_HYPERLANE_LOCAL_DOMAIN,
    hyperlane_types::{
        Addr32, IncrementalMerkleTree, addr32,
        constants::{ethereum, solana},
        mailbox::{self, MAILBOX_VERSION, Message},
    },
    sea_orm::EntityTrait,
};

#[test]
fn receiving_remote() {
    let (suite, mut accounts, _, contracts, validator_sets) = setup_test(Default::default());
    let mut suite = HyperlaneTestSuite::new(suite, validator_sets, &contracts);

    const MOCK_RECEIVE_AMOUNT: u128 = 88;

    let message_id = suite
        .receive_warp_transfer(
            &mut accounts.owner,
            solana::DOMAIN,
            solana::SOL_WARP,
            &accounts.user1,
            Uint128::new(MOCK_RECEIVE_AMOUNT),
        )
        .should_succeed();

    // The message should have been recorded as received.
    suite
        .query_wasm_smart(
            contracts.hyperlane.mailbox,
            mailbox::QueryDeliveredRequest { message_id },
        )
        .should_succeed_and_equal(true);

    // Alloyed synthetic tokens should have been minted to the receiver.
    suite
        .query_balance(&accounts.user1, sol::DENOM.clone())
        .should_succeed_and_equal(Uint128::new(MOCK_RECEIVE_AMOUNT));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sending_remote() {
    let (mut suite, mut accounts, _, contracts, _, context, ..) = setup_test_with_indexer().await;

    const RECIPIENT: Addr32 =
        addr32!("0000000000000000000000000000000000000000000000000000000000000000");

    const SEND_AMOUNT: u128 = 888_000_000;

    const ETHEREUM_USDC_WITHDRAWAL_FEE: u128 = 1_000_000;

    const SEND_AMOUNT_AFTER_FEE: u128 = SEND_AMOUNT - ETHEREUM_USDC_WITHDRAWAL_FEE;

    suite
        .balances()
        .record_many([&accounts.user1.address(), &contracts.taxman]);

    // User1 sends USDC to Ethereum.
    suite
        .execute(
            &mut accounts.user1,
            contracts.gateway,
            &gateway::ExecuteMsg::TransferRemote {
                remote: Remote::Warp {
                    domain: ethereum::DOMAIN,
                    contract: ethereum::USDC_WARP,
                },
                recipient: RECIPIENT,
            },
            coins! { usdc::DENOM.clone() => SEND_AMOUNT },
        )
        .should_succeed();

    // Message should have been inserted into the Merkle tree.
    suite
        .query_wasm_smart(contracts.hyperlane.mailbox, mailbox::QueryTreeRequest {})
        .should_succeed_and_equal({
            let token_msg = TokenMessage {
                recipient: RECIPIENT,
                amount: Uint128::new(SEND_AMOUNT_AFTER_FEE),
                metadata: Default::default(),
            };
            let msg = Message {
                version: MAILBOX_VERSION,
                nonce: 0,
                origin_domain: MOCK_HYPERLANE_LOCAL_DOMAIN,
                sender: contracts.warp.into(),
                destination_domain: ethereum::DOMAIN,
                recipient: ethereum::USDC_WARP,
                body: token_msg.encode(),
            };

            let mut tree = IncrementalMerkleTree::default();
            tree.insert(msg.encode().keccak256()).unwrap();
            tree
        });

    // Sender should have been deducted balance.
    suite.balances().should_change(&accounts.user1, btree_map! {
        usdc::DENOM.clone() => BalanceChange::Decreased(SEND_AMOUNT),
    });

    // Taxman should have received the fee.
    suite
        .balances()
        .should_change(&contracts.taxman, btree_map! {
            usdc::DENOM.clone() => BalanceChange::Increased(ETHEREUM_USDC_WITHDRAWAL_FEE),
        });

    // Gateway contract should not hold any of the synth token (should be burned).
    suite
        .query_balance(&contracts.gateway, usdc::DENOM.clone())
        .should_succeed_and_equal(Uint128::ZERO);

    // ----------------------------- Check indexer -----------------------------

    // Force the runtime to wait for the async indexer task to finish
    suite
        .app
        .indexer
        .wait_for_finish()
        .expect("Can't wait for indexer to finish");

    // The transfers should have been indexed.
    let blocks = indexer_sql::entity::blocks::Entity::find()
        .all(&context.db)
        .await
        .expect("Can't fetch blocks");

    assert_that!(blocks).has_length(1);

    let transfers = dango_indexer_sql::entity::transfers::Entity::find()
        .all(&context.db)
        .await
        .expect("Can't fetch transfers");

    // There should have been two transfers:
    // 1. Before fee amount from user to Gateway;
    // 2. Withdrawal fee from Gateway to taxman.
    assert_that!(transfers).has_length(2);

    assert_that!(
        transfers
            .iter()
            .map(|t| t.amount.as_str())
            .collect::<Vec<_>>()
    )
    .is_equal_to(vec![
        SEND_AMOUNT.to_string().as_str(),
        ETHEREUM_USDC_WITHDRAWAL_FEE.to_string().as_str(),
    ]);
}

#[test]
fn sending_remote_incorrect_route() {
    let (mut suite, mut accounts, _, contracts, ..) = setup_test(Default::default());

    const RECIPIENT: Addr32 =
        addr32!("0000000000000000000000000000000000000000000000000000000000000000");

    const ETHEREUM_WETH_REMOTE: Remote = Remote::Warp {
        domain: ethereum::DOMAIN,
        contract: ethereum::WETH_WARP, // Wrong!!
    };

    const SEND_AMOUNT: u128 = 888_000_000;

    // User attempts to send USDC through a wrong route (incorrct Warp address
    // on Ethereum). Should fail.
    suite
        .execute(
            &mut accounts.user1,
            contracts.gateway,
            &gateway::ExecuteMsg::TransferRemote {
                remote: ETHEREUM_WETH_REMOTE,
                recipient: RECIPIENT,
            },
            coins! { usdc::DENOM.clone() => SEND_AMOUNT },
        )
        .should_fail_with_error(StdError::data_not_found::<Addr>(
            REVERSE_ROUTES
                .path((&usdc::DENOM, ETHEREUM_WETH_REMOTE))
                .storage_key(),
        ));
}

#[test]
fn sending_remote_insufficient_reserve() {
    let (suite, mut accounts, _, contracts, validator_sets) = setup_test(Default::default());
    let mut suite = HyperlaneTestSuite::new(suite, validator_sets, &contracts);

    const MOCK_SOLANA_RECIPIENT: Addr32 =
        addr32!("0000000000000000000000000000000000000000000000000000000000000000");

    const SOLANA_USDC_REMOTE: Remote = Remote::Warp {
        domain: solana::DOMAIN,
        contract: solana::USDC_WARP,
    };

    const SEND_AMOUNT: u128 = 888_000_000;

    const SOLANA_USDC_WITHDRAWAL_FEE: u128 = 10_000;

    const SEND_AMOUNT_AFTER_FEE: u128 = SEND_AMOUNT - SOLANA_USDC_WITHDRAWAL_FEE;

    // Right now, the entire reserve of USDC is from Ethereum. User1 attempts to
    // withdraw to Solana. Should fail.
    suite
        .execute(
            &mut accounts.user1,
            contracts.gateway,
            &gateway::ExecuteMsg::TransferRemote {
                remote: SOLANA_USDC_REMOTE,
                recipient: MOCK_SOLANA_RECIPIENT,
            },
            coins! { usdc::DENOM.clone() => SEND_AMOUNT },
        )
        .should_fail_with_error(format!(
            "insufficient reserve! bridge: {}, remote: {:?}, reserve: {}, amount: {}",
            contracts.warp, SOLANA_USDC_REMOTE, 0, SEND_AMOUNT_AFTER_FEE
        ));

    // User2 receives some USDC so that we have sufficient reserve.
    suite
        .receive_warp_transfer(
            &mut accounts.owner,
            solana::DOMAIN,
            solana::USDC_WARP,
            &accounts.user2,
            Uint128::new(SEND_AMOUNT + 100), // A little more than sufficient amount.
        )
        .should_succeed();

    // User1 tries to withdraw again. Should succeed.
    suite
        .execute(
            &mut accounts.user1,
            contracts.gateway,
            &gateway::ExecuteMsg::TransferRemote {
                remote: SOLANA_USDC_REMOTE,
                recipient: MOCK_SOLANA_RECIPIENT,
            },
            coins! { usdc::DENOM.clone() => SEND_AMOUNT },
        )
        .should_succeed();
}
