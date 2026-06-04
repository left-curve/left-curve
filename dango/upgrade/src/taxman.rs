use {
    dango_bank::BALANCES,
    grug_app::{AppResult, CONFIG, CONTRACT_NAMESPACE, StorageProvider},
    grug_math::{Number, NumberConst, Uint128},
    grug_types::{Denom, Order, StdResult, Storage},
};

/// The taxman has accumulated protocol fees (tracked as token balances in the
/// bank contract) but has no message for withdrawing them. Move all of the
/// taxman's balances to the chain owner: credit each to the owner, then delete
/// the taxman's entry.
pub fn sweep_fees_to_owner(storage: Box<dyn Storage>) -> AppResult<()> {
    let cfg = CONFIG.load(&storage)?;

    // Scope storage to the bank contract, where token balances live.
    let mut bank_storage = StorageProvider::new(storage, &[CONTRACT_NAMESPACE, &cfg.bank]);

    // Collect the taxman's balances first, so the read borrow ends before we
    // start writing.
    let taxman_balances = BALANCES
        .prefix(&cfg.taxman)
        .range(&bank_storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<(Denom, Uint128)>>>()?;

    for (denom, amount) in taxman_balances {
        // The owner may already hold this denom, so add rather than overwrite.
        BALANCES.may_modify(
            &mut bank_storage,
            (&cfg.owner, &denom),
            |maybe| -> StdResult<_> {
                Ok(Some(maybe.unwrap_or(Uint128::ZERO).checked_add(amount)?))
            },
        )?;

        // Zero out the taxman's balance.
        BALANCES.remove(&mut bank_storage, (&cfg.taxman, &denom));
    }

    Ok(())
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        dango_testing::{TestOption, setup_test_naive},
        dango_types::constants::{eth, usdc},
        grug_app::AppError,
        grug_math::{IsZero, Number, NumberConst, Uint128},
        grug_types::{QuerierExt, ResultExt, coins},
    };

    /// On mainnet the taxman holds protocol fees in multiple tokens (ETH and
    /// USDC) while the owner holds only USDC. The chain upgrade must sweep every
    /// balance to the owner. We reproduce that here, so the sweep is exercised
    /// both on a denom the owner already holds (USDC, which must be merged) and
    /// one it doesn't (ETH, which must be created).
    #[tokio::test]
    async fn sweeping_taxman_fees_to_owner() {
        let usdc = usdc::DENOM.clone();
        let eth = eth::DENOM.clone();

        const USDC_SEED: u128 = 5_000_000_000;

        let (mut suite, mut accounts, ..) = setup_test_naive(TestOption::default());

        let taxman = suite.query_taxman().unwrap();

        // The owner is funded with both USDC and ETH at genesis. Move all of its
        // ETH, plus some USDC, into the taxman, so the taxman holds two tokens
        // while the owner is left holding only USDC. Funds are credited by
        // attaching them to an execute; `Configure` is the taxman's only message,
        // so we pass its current config back unchanged.
        let owner_eth = suite.query_balance(&accounts.owner, eth.clone()).unwrap();
        assert!(owner_eth.is_non_zero());
        let taxman_cfg = suite
            .query_wasm_smart(taxman, dango_types::taxman::QueryConfigRequest {})
            .unwrap();
        suite
            .execute(
                &mut accounts.owner,
                taxman,
                &dango_types::taxman::ExecuteMsg::Configure {
                    new_cfg: taxman_cfg,
                },
                coins! { usdc.clone() => Uint128::new(USDC_SEED), eth.clone() => owner_eth },
            )
            .await
            .should_succeed();

        // Schedule the upgrade, then advance to the block just before it.
        suite
            .upgrade(
                &mut accounts.owner,
                4,
                "0.1.0",
                None::<String>,
                None::<String>,
            )
            .await
            .should_succeed();
        suite.make_empty_block().await;

        // Record balances right before the upgrade. The taxman holds USDC (the
        // seed plus accrued gas fees) and ETH; the owner holds only USDC.
        let taxman_usdc = suite.query_balance(&taxman, usdc.clone()).unwrap();
        let taxman_eth = suite.query_balance(&taxman, eth.clone()).unwrap();
        let owner_usdc = suite.query_balance(&accounts.owner, usdc.clone()).unwrap();
        assert!(taxman_usdc.is_non_zero());
        assert_eq!(taxman_eth, owner_eth);
        assert_eq!(
            suite.query_balance(&accounts.owner, eth.clone()).unwrap(),
            Uint128::ZERO
        );

        // The chain halts on the version mismatch; install the real upgrade
        // handler and remake the block so the upgrade runs.
        suite.try_make_empty_block().await.should_fail_with_error(
            AppError::upgrade_incorrect_version("0.0.0".into(), "0.1.0".into()),
        );
        suite
            .app
            .set_cargo_version_and_upgrade_handler("0.1.0", Some(crate::do_upgrade));
        suite.make_empty_block().await;

        // The taxman has been emptied of both tokens; the owner received all of
        // it: USDC merged into its existing balance, ETH as a brand-new one.
        assert_eq!(
            suite.query_balance(&taxman, usdc.clone()).unwrap(),
            Uint128::ZERO
        );
        assert_eq!(
            suite.query_balance(&taxman, eth.clone()).unwrap(),
            Uint128::ZERO
        );
        assert_eq!(
            suite.query_balance(&accounts.owner, usdc).unwrap(),
            owner_usdc.checked_add(taxman_usdc).unwrap()
        );
        assert_eq!(
            suite.query_balance(&accounts.owner, eth).unwrap(),
            owner_eth
        );
    }
}
