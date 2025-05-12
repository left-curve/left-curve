use {
    assertor::*,
    dango_gateway::REVERSE_ROUTES,
    dango_genesis::Contracts,
    dango_testing::{TestSuite, setup_test, setup_test_with_indexer},
    dango_types::{
        constants::{dango, eth, sol, usdc},
        gateway::{self, Domain, Remote},
        warp::{self, TokenMessage},
    },
    grug::{
        Addr, Addressable, BalanceChange, Coin, Coins, Denom, Duration, Hash256, HashExt,
        HexBinary, JsonSerExt, MathError, NumberConst, QuerierExt, ResultExt, Signer, StdError,
        Udec128, Uint128, btree_map, coins, setup_tracing_subscriber,
    },
    hyperlane_testing::{MockValidatorSets, constants::MOCK_HYPERLANE_LOCAL_DOMAIN},
    hyperlane_types::{
        Addr32, IncrementalMerkleTree, addr32,
        constants::{ethereum, solana},
        mailbox::{self, MAILBOX_VERSION, Message},
    },
    sea_orm::EntityTrait,
    std::{
        ops::{Deref, DerefMut},
        str::FromStr,
    },
};

struct WarpTestSuite {
    suite: TestSuite,
    validator_sets: MockValidatorSets,
    mailbox: Addr,
    warp: Addr,
}

impl Deref for WarpTestSuite {
    type Target = TestSuite;

    fn deref(&self) -> &Self::Target {
        &self.suite
    }
}

impl DerefMut for WarpTestSuite {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.suite
    }
}

impl WarpTestSuite {
    fn new(suite: TestSuite, validator_sets: MockValidatorSets, contracts: &Contracts) -> Self {
        Self {
            suite,
            validator_sets,
            mailbox: contracts.hyperlane.mailbox,
            warp: contracts.warp,
        }
    }

    fn receive_warp_transfer(
        &mut self,
        relayer: &mut dyn Signer,
        origin_domain: Domain,
        origin_warp: Addr32,
        recipient: Addr,
        amount: Uint128,
    ) -> Hash256 {
        // Mock validator set signs the message.
        let (message_id, raw_message, raw_metadata) = self.validator_sets.get(origin_domain).sign(
            origin_warp,
            MOCK_HYPERLANE_LOCAL_DOMAIN,
            self.warp,
            TokenMessage {
                recipient: recipient.into(),
                amount,
                metadata: Default::default(),
            }
            .encode(),
        );

        // Deliver the message to Dango mailbox.
        self.suite
            .execute(
                relayer,
                self.mailbox,
                &mailbox::ExecuteMsg::Process {
                    raw_message,
                    raw_metadata,
                },
                Coins::new(),
            )
            .should_succeed();

        // Return the message ID.
        message_id
    }
}

#[test]
fn receiving_remote() {
    let (suite, mut accounts, _, contracts, validator_sets) = setup_test(Default::default());
    let mut suite = WarpTestSuite::new(suite, validator_sets, &contracts);

    const MOCK_RECEIVE_AMOUNT: u128 = 88;

    let message_id = suite.receive_warp_transfer(
        &mut accounts.owner,
        solana::DOMAIN,
        solana::SOL_WARP,
        accounts.user1.address(),
        Uint128::new(MOCK_RECEIVE_AMOUNT),
    );

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

#[test]
fn sending_remote() {
    let (mut suite, mut accounts, _, contracts, ..) = setup_test_with_indexer();

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
    suite.app.indexer.wait_for_finish();

    // The transfers should have been indexed.
    suite.app.indexer.handle.block_on(async {
        let blocks = indexer_sql::entity::blocks::Entity::find()
            .all(&suite.app.indexer.context.db)
            .await
            .expect("Can't fetch blocks");

        assert_that!(blocks).has_length(1);

        let transfers = dango_indexer_sql::entity::transfers::Entity::find()
            .all(&suite.app.indexer.context.db)
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
    });
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
    let mut suite = WarpTestSuite::new(suite, validator_sets, &contracts);

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
    suite.receive_warp_transfer(
        &mut accounts.owner,
        solana::DOMAIN,
        solana::USDC_WARP,
        accounts.user2.address(),
        Uint128::new(SEND_AMOUNT + 100), // A little more than sufficient amount.
    );

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

#[test]
fn rate_limit() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test(Default::default());

    suite.block_time = Duration::ZERO;

    let osmo_domain = 10;
    let sol_domain = 20;

    let osmo_usdc_recipient = Addr::mock(1).into();
    let sol_usdc_recipient = Addr::mock(2).into();
    let mock_remote_user = Addr::mock(3).into();

    let sol_usdc_denom = Denom::from_str("hyp/sol/usdc").unwrap();
    let osmo_usdc_denom = Denom::from_str("hyp/osmo/usdc").unwrap();

    let alloyed_usdc_denom = Denom::from_str("hyp/all/usdc").unwrap();

    let (mut suite, owner) = HyperlaneTestSuite::new(suite, accounts.owner, btree_map! {
        osmo_domain => (3, 2),
        sol_domain => (3, 2),
    });

    // Set the route.
    for (domain, denom, route) in [
        (osmo_domain, &osmo_usdc_denom, Route {
            address: osmo_usdc_recipient,
            fee: Uint128::ZERO,
        }),
        (sol_domain, &sol_usdc_denom, Route {
            address: sol_usdc_recipient,
            fee: Uint128::ZERO,
        }),
    ] {
        suite
            .hyperlane()
            .set_route(denom.clone(), domain, route)
            .should_succeed();
    }

    suite.balances().record(accounts.user1.address());

    // Receive some tokens.
    // osmo_usdc => 100
    // sol_usdc => 200
    {
        for (domain, denom, amount) in [
            (osmo_domain, &osmo_usdc_denom, 100),
            (sol_domain, &sol_usdc_denom, 200),
        ] {
            suite.hyperlane().receive_transfer(
                domain,
                accounts.user1.address(),
                coin(denom, amount),
            );
        }

        // Check balances.
        suite
            .balances()
            .should_change(accounts.user1.address(), btree_map! {
                osmo_usdc_denom.clone() => BalanceChange::Increased(100),
                sol_usdc_denom.clone() => BalanceChange::Increased(200),
            });
    }

    // Set rate limit.
    // osmo_usdc => 10%
    // sol_usdc => 20%
    suite
        .execute(
            owner.write_access().deref_mut(),
            contracts.warp,
            &warp::ExecuteMsg::SetRateLimits(btree_map! {
                osmo_usdc_denom.clone() => RateLimit::new_unchecked(Udec128::new_percent(10)),
                sol_usdc_denom.clone()  => RateLimit::new_unchecked(Udec128::new_percent(20)),
            }),
            Coins::default(),
        )
        .should_succeed();

    // Make 1 day pass letting the cron job to reset the rate limits.
    advance_to_next_day(&mut suite);

    // Try send back exact tokens to don't trigger rate limit.
    // osmo_usdc => 100 * 0.1 = 10
    // sol_usdc => 200 * 0.2 = 40
    for (domain, denom, amount) in [
        (osmo_domain, &osmo_usdc_denom, 10),
        (sol_domain, &sol_usdc_denom, 40),
    ] {
        suite
            .hyperlane()
            .send_transfer(
                &mut accounts.user1,
                domain,
                mock_remote_user,
                coin(denom, amount),
            )
            .should_succeed();
    }

    // Trigger the rate limit sending 1 more token.
    for (domain, denom, amount) in [
        (osmo_domain, &osmo_usdc_denom, 1),
        (sol_domain, &sol_usdc_denom, 1),
    ] {
        suite
            .hyperlane()
            .send_transfer(
                &mut accounts.user1,
                domain,
                mock_remote_user,
                coin(denom, amount),
            )
            .should_fail_with_error("rate limit reached: 0 < 1");
    }

    // Receive more tokens increase rate limit and allow to send them back.
    for (domain, denom, amount) in [
        (osmo_domain, &osmo_usdc_denom, 100),
        (sol_domain, &sol_usdc_denom, 200),
    ] {
        suite
            .hyperlane()
            .receive_transfer(domain, accounts.user1.address(), coin(denom, amount));

        suite
            .hyperlane()
            .send_transfer(
                &mut accounts.user1,
                domain,
                mock_remote_user,
                coin(denom, amount),
            )
            .should_succeed();
    }

    // Make 1 day pass letting the cron job to reset the rate limits.
    advance_to_next_day(&mut suite);

    // The supply on chain now are:
    // osmo_usdc => 100 - 10 = 90
    // sol_usdc => 200 - 40 = 160

    // New limits are:
    // osmo_usdc => 90 * 0.1 = 9
    // sol_usdc => 160 * 0.2 = 32

    // Try send them back.
    for (domain, denom, amount) in [
        (osmo_domain, &osmo_usdc_denom, 9),
        (sol_domain, &sol_usdc_denom, 32),
    ] {
        suite
            .hyperlane()
            .send_transfer(
                &mut accounts.user1,
                domain,
                mock_remote_user,
                coin(denom, amount),
            )
            .should_succeed();
    }

    // Trigger the rate limit sending 1 more token.
    for (domain, denom, amount) in [
        (osmo_domain, &osmo_usdc_denom, 1),
        (sol_domain, &sol_usdc_denom, 1),
    ] {
        suite
            .hyperlane()
            .send_transfer(
                &mut accounts.user1,
                domain,
                mock_remote_user,
                coin(denom, amount),
            )
            .should_fail_with_error("rate limit reached: 0 < 1");
    }

    // Receive some tokens to reset back the supply to:
    // osmo_usdc => 100
    // sol_usdc => 200
    for (domain, denom, amount, should_be) in [
        (osmo_domain, &osmo_usdc_denom, 10 + 9, 100),
        (sol_domain, &sol_usdc_denom, 40 + 32, 200),
    ] {
        suite
            .hyperlane()
            .receive_transfer(domain, accounts.user1.address(), coin(denom, amount));

        suite
            .query_supply(denom.clone())
            .should_succeed_and_equal(Uint128::new(should_be));
    }

    // Create alloy token
    for (domain, denom) in [
        (osmo_domain, &osmo_usdc_denom),
        (sol_domain, &sol_usdc_denom),
    ] {
        suite
            .execute(
                owner.write_access().deref_mut(),
                contracts.warp,
                &warp::ExecuteMsg::SetAlloy {
                    underlying_denom: denom.clone(),
                    destination_domain: domain,
                    alloyed_denom: alloyed_usdc_denom.clone(),
                },
                Coins::default(),
            )
            .should_succeed();
    }

    // Receive some tokens. they should be minted as alloyed:
    // osmo_usdc => 100
    // sol_usdc  => 200
    for (domain, denom, amount) in [
        (osmo_domain, &osmo_usdc_denom, 100),
        (sol_domain, &sol_usdc_denom, 200),
    ] {
        suite
            .hyperlane()
            .receive_transfer(domain, accounts.user1.address(), coin(denom, amount));
    }

    // Make 1 day pass letting the cron job to reset the rate limits.
    advance_to_next_day(&mut suite);

    // The supply on chain now are:
    // osmo_usdc => 100 + 100 = 200
    // sol_usdc  => 200 + 200 = 400

    // Send the alloyed tokens.
    for (domain, denom, amount) in [
        (osmo_domain, &alloyed_usdc_denom, 20),
        (sol_domain, &alloyed_usdc_denom, 80),
    ] {
        suite
            .hyperlane()
            .send_transfer(
                &mut accounts.user1,
                domain,
                mock_remote_user,
                coin(denom, amount),
            )
            .should_succeed();
    }

    // Send 1 more tokens to trigger the rate limit.
    for (domain, denom, amount) in [
        (osmo_domain, &alloyed_usdc_denom, 1),
        (sol_domain, &alloyed_usdc_denom, 1),
    ] {
        suite
            .hyperlane()
            .send_transfer(
                &mut accounts.user1,
                domain,
                mock_remote_user,
                coin(denom, amount),
            )
            .should_fail_with_error("rate limit reached: 0 < 1");
    }

    // Try send 1 of the underlying tokens. Rate limit should be triggered.
    for (domain, denom, amount) in [
        (osmo_domain, &osmo_usdc_denom, 1),
        (sol_domain, &sol_usdc_denom, 1),
    ] {
        suite
            .hyperlane()
            .send_transfer(
                &mut accounts.user1,
                domain,
                mock_remote_user,
                coin(denom, amount),
            )
            .should_fail_with_error("rate limit reached: 0 < 1");
    }
}

fn advance_to_next_day(suite: &mut TestSuite) {
    suite.block_time = Duration::from_days(1);
    suite.make_empty_block();
    suite.block_time = Duration::ZERO;
}

fn coin(denom: &Denom, amount: u128) -> Coin {
    Coin::new(denom.clone(), amount).unwrap()
}
