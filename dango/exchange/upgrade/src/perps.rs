use {
    dango_app::{AppResult, CHAIN_ID, CONTRACT_NAMESPACE, StorageProvider},
    dango_bank::BALANCES,
    dango_math::{NumberConst, Uint128},
    dango_order_book::{Quantity, UsdValue},
    dango_perps::{
        core::compute_user_equity,
        querier::NoCachePerpQuerier,
        state::{STATE, USER_STATES},
    },
    dango_primitives::{Addr, Order, StdError, StdResult, Storage, addr},
    dango_types::perps::{SETTLEMENT_CURRENCY_PRICE, settlement_currency},
};

const MAINNET_CHAIN_ID: &str = "dango-1";
const MAINNET_PERPS_ADDRESS: Addr = addr!("90bc84df68d1aa59a857e04ed529e9a26edbea4f");

const TESTNET_CHAIN_ID: &str = "dango-testnet-1";
const TESTNET_PERPS_ADDRESS: Addr = addr!("f6344c5e2792e8f9202c58a2d88fbbde4cd3142f");

/// The bank contract address. Identical on mainnet and testnet.
const BANK_ADDRESS: Addr = addr!("e0b49f70991ecab05d5d7dc1f71e4ede63c8f2b7");

pub fn do_perps_upgrades(storage: Box<dyn Storage>) -> AppResult<()> {
    let perps_address = {
        let chain_id = CHAIN_ID.load(&storage)?;
        match chain_id.as_str() {
            MAINNET_CHAIN_ID => MAINNET_PERPS_ADDRESS,
            TESTNET_CHAIN_ID => TESTNET_PERPS_ADDRESS,
            _ => panic!("unknown chain id: {chain_id}"),
        }
    };

    // Two storage handles over the same underlying (Arc-backed) buffer: one
    // scoped to the perps contract (read state, write the adjusted insurance
    // fund), one scoped to the bank contract (read the perps' USDC balance).
    let mut perps_storage =
        StorageProvider::new(storage.clone(), &[CONTRACT_NAMESPACE, &perps_address]);
    let bank_storage = StorageProvider::new(storage, &[CONTRACT_NAMESPACE, &BANK_ADDRESS]);

    restore_insurance_fund_solvency(&mut perps_storage, &bank_storage, perps_address)
        // `?` converts `StdError` into `AppError`; there is no
        // `From<anyhow::Error>`, so bridge through `StdError::host`.
        .map_err(|err| StdError::host(err.to_string()))?;

    Ok(())
}

/// Restore the perps exchange's solvency by deducting the deficit from the
/// insurance fund.
///
/// The exchange is solvent when its USDC balance covers all obligations:
///
/// ```plain
/// USDC balance >= Σ equity + Σ unlocks + treasury + insurance_fund
/// ```
///
/// Due to a historical event, the right-hand side currently exceeds the left by
/// a small deficit. This deducts exactly that deficit from the insurance fund,
/// bringing the exchange to break-even solvency.
///
/// - If the exchange is already solvent (no deficit), this is a no-op.
/// - If the insurance fund cannot cover the deficit, this logs an error and is a
///   no-op (it does _not_ return an error, which would halt the chain).
fn restore_insurance_fund_solvency(
    perps_storage: &mut dyn Storage,
    bank_storage: &dyn Storage,
    perps_address: Addr,
) -> anyhow::Result<()> {
    // Sum equity and pending unlocks across all users. The market-making vault
    // is itself a user (keyed by the perps contract's own address), so it is
    // included here exactly once.
    let user_states = USER_STATES
        .range(perps_storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()?;

    // Sum in a scope so the querier's immutable borrow of `perps_storage` is
    // released before the mutable write further below.
    let (total_equity, total_unlock) = {
        let querier = NoCachePerpQuerier::new_local(perps_storage);

        let mut total_equity = UsdValue::ZERO;
        let mut total_unlock = UsdValue::ZERO;

        for (_addr, user_state) in &user_states {
            total_equity.checked_add_assign(compute_user_equity(&querier, user_state)?)?;

            for unlock in &user_state.unlocks {
                total_unlock.checked_add_assign(unlock.amount_to_release)?;
            }
        }

        (total_equity, total_unlock)
    };

    let mut state = STATE.load(perps_storage)?;

    // The perps contract's USDC balance, valued in USD (settlement currency is
    // pegged 1:1).
    let usdc = settlement_currency::DENOM.clone();
    let balance_raw = BALANCES
        .may_load(bank_storage, (&perps_address, &usdc))?
        .unwrap_or(Uint128::ZERO);
    let balance = Quantity::from_base(balance_raw, settlement_currency::DECIMAL)?
        .checked_mul(SETTLEMENT_CURRENCY_PRICE)?;

    let liability = total_equity
        .checked_add(total_unlock)?
        .checked_add(state.treasury)?
        .checked_add(state.insurance_fund)?;

    let deficit = liability.checked_sub(balance)?;

    if deficit <= UsdValue::ZERO {
        tracing::info!(
            balance = %balance,
            liability = %liability,
            "perps exchange is already solvent; no insurance fund adjustment needed"
        );

        return Ok(());
    }

    if state.insurance_fund < deficit {
        tracing::error!(
            deficit = %deficit,
            insurance_fund = %state.insurance_fund,
            "insurance fund is insufficient to cover the solvency deficit; no-op"
        );

        return Ok(());
    }

    let old_insurance_fund = state.insurance_fund;
    state.insurance_fund = state.insurance_fund.checked_sub(deficit)?;
    STATE.save(perps_storage, &state)?;

    tracing::info!(
        deficit = %deficit,
        old_insurance_fund = %old_insurance_fund,
        new_insurance_fund = %state.insurance_fund,
        "restored perps exchange solvency by deducting the deficit from the insurance fund"
    );

    Ok(())
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        dango_primitives::{Binary, MockStorage, Shared},
        dango_types::perps::{State, UserState},
        std::{collections::BTreeMap, fs, path::PathBuf},
    };

    const PERPS: Addr = Addr::mock(1);
    const USER: Addr = Addr::mock(2);

    /// Build perps + bank storage handles backed by one shared in-memory store,
    /// seeded with a single position-less user (whose equity is just its margin),
    /// a `State`, and the perps contract's USDC bank balance.
    fn setup(
        insurance_fund: i128,
        treasury: i128,
        user_equity: i128,
        usdc_balance_base: u128,
    ) -> (StorageProvider, StorageProvider) {
        let base = Shared::new(MockStorage::new());

        let mut perps = StorageProvider::new(Box::new(base.clone()), &[CONTRACT_NAMESPACE, &PERPS]);
        let mut bank =
            StorageProvider::new(Box::new(base.clone()), &[CONTRACT_NAMESPACE, &BANK_ADDRESS]);

        STATE
            .save(&mut perps, &State {
                insurance_fund: UsdValue::new_int(insurance_fund),
                treasury: UsdValue::new_int(treasury),
                ..Default::default()
            })
            .unwrap();

        USER_STATES
            .save(&mut perps, USER, &UserState {
                margin: UsdValue::new_int(user_equity),
                ..Default::default()
            })
            .unwrap();

        BALANCES
            .save(
                &mut bank,
                (&PERPS, &settlement_currency::DENOM.clone()),
                &Uint128::new(usdc_balance_base),
            )
            .unwrap();

        (perps, bank)
    }

    // balance 100, equity 90, treasury 5, insurance 20
    // liability = 90 + 5 + 20 = 115; deficit = 15; 20 >= 15 -> deduct -> 5
    // afterwards: balance 100 == equity 90 + treasury 5 + insurance 5 (break-even)
    #[test]
    fn deducts_exact_deficit_when_sufficient() {
        let (mut perps, bank) = setup(20, 5, 90, 100_000_000);

        restore_insurance_fund_solvency(&mut perps, &bank, PERPS).unwrap();

        let state = STATE.load(&perps).unwrap();
        assert_eq!(state.insurance_fund, UsdValue::new_int(5));
        assert_eq!(state.treasury, UsdValue::new_int(5));
    }

    // balance 100, equity 200, treasury 5, insurance 10
    // liability = 215; deficit = 115 > 10 -> no-op
    #[test]
    fn no_op_when_insufficient() {
        let (mut perps, bank) = setup(10, 5, 200, 100_000_000);

        restore_insurance_fund_solvency(&mut perps, &bank, PERPS).unwrap();

        let state = STATE.load(&perps).unwrap();
        assert_eq!(state.insurance_fund, UsdValue::new_int(10));
        assert_eq!(state.treasury, UsdValue::new_int(5));
    }

    // balance 100, equity 50, treasury 5, insurance 10
    // liability = 65; deficit = -35 <= 0 -> no-op
    #[test]
    fn no_op_when_already_solvent() {
        let (mut perps, bank) = setup(10, 5, 50, 100_000_000);

        restore_insurance_fund_solvency(&mut perps, &bank, PERPS).unwrap();

        let state = STATE.load(&perps).unwrap();
        assert_eq!(state.insurance_fund, UsdValue::new_int(10));
        assert_eq!(state.treasury, UsdValue::new_int(5));
    }

    // ----------------------------- real-data tests ---------------------------
    //
    // These load real perps storage + USDC balance snapshots, pulled from the
    // live chains into `testdata/` (gitignored, as the data is large and
    // chain-state-dependent). The tests are ignored by default; run them with:
    //
    //   cargo test -p dango-upgrade -- --ignored
    //
    // Each chain needs two fixtures: `testdata/<chain>_perps.json`, a `wasm_scan`
    // dump of the perps contract's `state`, `us`, and `pair_state` storage as a
    // `{base64 key: base64 value}` map; and `testdata/<chain>_balance.json`, the
    // perps contract's USDC balance in base units.

    fn testdata(name: &str, kind: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("testdata")
            .join(format!("{name}_{kind}.json"))
    }

    /// Rebuild a chain's perps + bank storage in memory from the dumped fixtures.
    fn reconstruct(name: &str, perps_address: Addr) -> Shared<MockStorage> {
        let base = Shared::new(MockStorage::new());

        // Perps storage: write each scanned (contract-relative) key/value back
        // through a provider scoped to the perps contract.
        let perps_dump: BTreeMap<Binary, Binary> =
            serde_json::from_slice(&fs::read(testdata(name, "perps")).unwrap()).unwrap();
        let mut perps = StorageProvider::new(Box::new(base.clone()), &[
            CONTRACT_NAMESPACE,
            &perps_address,
        ]);
        for (key, value) in &perps_dump {
            perps.write(key.as_ref(), value.as_ref());
        }

        // Bank storage: only the perps contract's USDC balance is needed.
        let balance: Uint128 =
            serde_json::from_slice(&fs::read(testdata(name, "balance")).unwrap()).unwrap();
        let mut bank =
            StorageProvider::new(Box::new(base.clone()), &[CONTRACT_NAMESPACE, &BANK_ADDRESS]);
        BALANCES
            .save(
                &mut bank,
                (&perps_address, &settlement_currency::DENOM.clone()),
                &balance,
            )
            .unwrap();

        base
    }

    /// Run the solvency fix against a reconstructed chain snapshot, assert the
    /// universal postconditions, and return the amount deducted.
    fn restore_solvency_real(name: &str, perps_address: Addr) -> UsdValue {
        let base = reconstruct(name, perps_address);

        let mut perps = StorageProvider::new(Box::new(base.clone()), &[
            CONTRACT_NAMESPACE,
            &perps_address,
        ]);
        let bank =
            StorageProvider::new(Box::new(base.clone()), &[CONTRACT_NAMESPACE, &BANK_ADDRESS]);

        let before = STATE.load(&perps).unwrap();

        restore_insurance_fund_solvency(&mut perps, &bank, perps_address).unwrap();

        let after = STATE.load(&perps).unwrap();

        let deducted = before
            .insurance_fund
            .checked_sub(after.insurance_fund)
            .unwrap();
        println!(
            "{name}: insurance fund {} -> {} (deducted {})",
            before.insurance_fund, after.insurance_fund, deducted,
        );

        // The treasury is never touched.
        assert_eq!(after.treasury, before.treasury);

        // Solvency is now restored: running again finds no deficit and is a
        // no-op (the insurance fund is unchanged on the second pass).
        restore_insurance_fund_solvency(&mut perps, &bank, perps_address).unwrap();
        let after_second = STATE.load(&perps).unwrap();
        assert_eq!(after_second.insurance_fund, after.insurance_fund);

        deducted
    }

    #[test]
    #[ignore = "requires gitignored testdata/mainnet_{perps,balance}.json fixtures"]
    fn restore_solvency_mainnet() {
        let deducted = restore_solvency_real("mainnet", MAINNET_PERPS_ADDRESS);

        // The known historical deficit is ~$3643. Allow a margin for the small
        // drift between snapshots taken at different times.
        assert!(deducted > UsdValue::new_int(3_000) && deducted < UsdValue::new_int(4_500));
    }

    #[test]
    #[ignore = "requires gitignored testdata/testnet_{perps,balance}.json fixtures"]
    fn restore_solvency_testnet() {
        // Testnet's deficit is not a known fixed figure; the helper already
        // asserts the function runs, restores solvency, and leaves the treasury
        // untouched.
        let _ = restore_solvency_real("testnet", TESTNET_PERPS_ADDRESS);
    }
}
