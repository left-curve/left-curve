use {
    dango_testing::setup_test_naive,
    dango_types::{
        account::{self, ExecuteMsg},
        auth::AccountStatus,
        constants::dango,
    },
    grug_types::{Addressable, Coins, QuerierExt, ResultExt},
};

/// The chain's owner can freeze an active account, blocking it from sending
/// transactions and receiving transfers. Unfreezing restores normal behavior.
#[tokio::test]
async fn freeze_and_unfreeze_lifecycle() {
    let (mut suite, mut accounts, ..) = setup_test_naive(Default::default());

    let target = accounts.user1.address();

    // Sanity check: user1 starts in the `Active` state.
    suite
        .query_wasm_smart(target, account::QueryStatusRequest {})
        .should_succeed_and_equal(AccountStatus::Active);

    // Owner freezes user1's account.
    suite
        .execute(
            &mut accounts.owner,
            target,
            &ExecuteMsg::Freeze {},
            Coins::new(),
        )
        .await
        .should_succeed();

    suite
        .query_wasm_smart(target, account::QueryStatusRequest {})
        .should_succeed_and_equal(AccountStatus::Frozen);

    // user1 can no longer send transactions.
    suite
        .transfer(
            &mut accounts.user1,
            accounts.user2.address(),
            Coins::one(dango::DENOM.clone(), 100).unwrap(),
        )
        .await
        .should_fail_with_error(format!("account {target} is not active"));

    // user1 can no longer receive transfers.
    suite
        .transfer(
            &mut accounts.user2,
            target,
            Coins::one(dango::DENOM.clone(), 100).unwrap(),
        )
        .await
        .should_fail_with_error(format!("account {target} is frozen"));

    // Owner unfreezes user1's account.
    suite
        .execute(
            &mut accounts.owner,
            target,
            &ExecuteMsg::Unfreeze {},
            Coins::new(),
        )
        .await
        .should_succeed();

    suite
        .query_wasm_smart(target, account::QueryStatusRequest {})
        .should_succeed_and_equal(AccountStatus::Active);

    // user1 can send and receive again.
    suite
        .transfer(
            &mut accounts.user1,
            accounts.user2.address(),
            Coins::one(dango::DENOM.clone(), 100).unwrap(),
        )
        .await
        .should_succeed();

    suite
        .transfer(
            &mut accounts.user2,
            target,
            Coins::one(dango::DENOM.clone(), 100).unwrap(),
        )
        .await
        .should_succeed();
}

/// Only the chain's owner can freeze an account. A non-owner sender is
/// rejected with the standard authorization error.
#[tokio::test]
async fn non_owner_cannot_freeze() {
    let (mut suite, mut accounts, ..) = setup_test_naive(Default::default());

    let target = accounts.user1.address();

    suite
        .execute(
            &mut accounts.user2,
            target,
            &ExecuteMsg::Freeze {},
            Coins::new(),
        )
        .await
        .should_fail_with_error("you don't have the right, O you don't have the right");

    // Status should be unchanged.
    suite
        .query_wasm_smart(target, account::QueryStatusRequest {})
        .should_succeed_and_equal(AccountStatus::Active);
}

/// Freezing is idempotent: re-freezing an already-frozen account is a no-op
/// and leaves the status unchanged.
#[tokio::test]
async fn freeze_is_idempotent() {
    let (mut suite, mut accounts, ..) = setup_test_naive(Default::default());

    let target = accounts.user1.address();

    suite
        .execute(
            &mut accounts.owner,
            target,
            &ExecuteMsg::Freeze {},
            Coins::new(),
        )
        .await
        .should_succeed();
    suite
        .execute(
            &mut accounts.owner,
            target,
            &ExecuteMsg::Freeze {},
            Coins::new(),
        )
        .await
        .should_succeed();

    suite
        .query_wasm_smart(target, account::QueryStatusRequest {})
        .should_succeed_and_equal(AccountStatus::Frozen);
}

/// Unfreezing an account that isn't `Frozen` (e.g. already active) is rejected.
#[tokio::test]
async fn unfreeze_requires_frozen_status() {
    let (mut suite, mut accounts, ..) = setup_test_naive(Default::default());

    let target = accounts.user1.address();

    suite
        .execute(
            &mut accounts.owner,
            target,
            &ExecuteMsg::Unfreeze {},
            Coins::new(),
        )
        .await
        .should_fail_with_error("can only unfreeze a frozen account");
}
