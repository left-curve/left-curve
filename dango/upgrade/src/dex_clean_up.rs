//! Dango has pivoted to being exclusively a perpetual futures exchange. The
//! spot DEX contract was removed from the codebase, but traces of it remain
//! in the chain state:
//!
//! - the bank contract records balances and supplies of the DEX's liquidity
//!   share tokens (denoms under the `dex/pool` namespace);
//! - the bank contract records the now-dead DEX contract as the owner of the
//!   `dex` token namespace;
//! - the DEX contract holds dust amounts of various tokens;
//! - the DEX contract's own storage still contains a few records.
//!
//! This migration removes all of the above. LP share records are deleted
//! without compensation (on testnet, where users still hold LP shares, this
//! is accepted); the DEX's remaining token balances are credited to the
//! chain owner.

use {
    dango_bank::{BALANCES, NAMESPACE_OWNERS, SUPPLIES},
    grug_app::{AppResult, CONFIG, CONTRACT_NAMESPACE, StorageProvider},
    grug_math::{IsZero, Number, NumberConst, Uint128},
    grug_types::{Addr, Order, Part, Shared, StdError, StdResult, Storage, addr},
};

/// Derived at genesis from the deployer, code hash, and the salt `dango/dex`;
/// identical on mainnet and testnet.
const DEX_ADDRESS: Addr = addr!("da32476efe31e535207f0ad690d337a4ebf54a22");

pub fn clean_up_dex(storage: Box<dyn Storage>) -> AppResult<()> {
    // The single storage handle is needed by two `StorageProvider`s (bank and
    // dex substores) as well as for app-level reads, so wrap it in `Shared`.
    let storage = Shared::new(storage);

    let config = CONFIG.load(&storage)?;

    let mut bank_storage = StorageProvider::new(Box::new(storage.clone()), &[
        CONTRACT_NAMESPACE,
        &config.bank,
    ]);

    let lp_prefix = [Part::new_unchecked("dex"), Part::new_unchecked("pool")];

    // 1. Delete all balances of LP share tokens, for all holders. This
    //    includes the DEX contract's own LP balance, so that step 3 below
    //    never credits LP shares to the owner.
    let lp_balances = BALANCES
        .keys(&bank_storage, None, None, Order::Ascending)
        .filter_map(|res| match res {
            Ok((address, denom)) => denom
                .starts_with(&lp_prefix)
                .then_some(Ok((address, denom))),
            Err(err) => Some(Err(err)),
        })
        .collect::<StdResult<Vec<_>>>()?;

    for (address, denom) in &lp_balances {
        BALANCES.remove(&mut bank_storage, (address, denom));

        tracing::info!(%address, %denom, "Removed balance record");
    }

    // 2. Delete all supplies of LP share tokens.
    let lp_supplies = SUPPLIES
        .keys(&bank_storage, None, None, Order::Ascending)
        .filter_map(|res| match res {
            Ok(denom) => denom.starts_with(&lp_prefix).then_some(Ok(denom)),
            Err(err) => Some(Err(err)),
        })
        .collect::<StdResult<Vec<_>>>()?;

    for denom in &lp_supplies {
        SUPPLIES.remove(&mut bank_storage, denom);

        tracing::info!(%denom, "Removed supply record");
    }

    // 3. Credit all remaining balances of the DEX contract (dust left behind
    //    by rounding in swaps and liquidity withdrawals) to the chain owner.
    //    Supplies are unchanged: this is a transfer, not a burn.
    let swept = BALANCES
        .prefix(&DEX_ADDRESS)
        .drain(&mut bank_storage, None, None)?;

    for (denom, amount) in &swept {
        if amount.is_zero() {
            continue;
        }

        let balance = BALANCES
            .may_load(&bank_storage, (&config.owner, denom))?
            .unwrap_or(Uint128::ZERO)
            .checked_add(*amount)
            .map_err(StdError::from)?;

        BALANCES.save(&mut bank_storage, (&config.owner, denom), &balance)?;

        tracing::info!(%denom, %amount, "Swept DEX balance to owner");
    }

    // 4. Delete the record of the DEX contract being the owner of the `dex`
    //    token namespace.
    let dex_namespace = Part::new_unchecked("dex");
    if NAMESPACE_OWNERS.may_load(&bank_storage, &dex_namespace)? == Some(DEX_ADDRESS) {
        NAMESPACE_OWNERS.remove(&mut bank_storage, &dex_namespace);

        tracing::info!("Removed DEX as namespace owner");
    }

    // 5. Wipe the DEX contract's own storage.
    let mut dex_storage = StorageProvider::new(Box::new(storage.clone()), &[
        CONTRACT_NAMESPACE,
        &DEX_ADDRESS,
    ]);

    let dex_records = dex_storage.scan_keys(None, None, Order::Ascending).count();
    dex_storage.remove_range(None, None);

    tracing::info!(
        lp_balances_deleted = lp_balances.len(),
        lp_supplies_deleted = lp_supplies.len(),
        swept = swept
            .iter()
            .map(|(denom, amount)| format!("{amount}{denom}"))
            .collect::<Vec<_>>()
            .join(","),
        dex_records_deleted = dex_records,
        owner = %config.owner,
        "cleaned up dex residue"
    );

    Ok(())
}

// ----------------------------------- tests -----------------------------------

/// The tests run the migration against the full raw states of the bank and
/// DEX contracts pulled from the live chains, and are `#[ignore]`d because
/// those fixtures are too large to commit (the testnet bank dump alone is
/// ~100 MiB). To run them:
///
/// 1. Place the following files in `dango/upgrade/testdata/` (gitignored):
///
///    - `bank-state-mainnet.json`, `bank-state-testnet.json`: full state of
///      the bank contract (`0xe0b49f70991ecab05d5d7dc1f71e4ede63c8f2b7` on
///      both chains);
///    - `dex-state-mainnet.json`, `dex-state-testnet.json`: full state of the
///      DEX contract (`0xda32476efe31e535207f0ad690d337a4ebf54a22`).
///
///    Each file is a JSON object of base64 key => base64 value, obtained by
///    POSTing the GraphQL query below to `https://api-mainnet.dango.zone/graphql`
///    (resp. `api-testnet`) repeatedly until a page comes back empty, merging
///    the pages; `$min` starts as null, then is the previous page's last key
///    with a zero byte appended (base64):
///
///    ```graphql
///    query($request: GrugQueryInput!) { queryApp(request: $request) }
///    # variables:
///    # {"request": {"wasm_scan": {"contract": "<addr>", "limit": 1000, "min": $min}}}
///    ```
///
/// 2. `cargo test -p dango-upgrade -- --ignored`
#[cfg(test)]
mod tests {
    use {
        super::*,
        grug_types::{Binary, Config, Duration, MockStorage, Permission, Permissions},
        std::{collections::BTreeMap, fs, path::PathBuf},
    };

    /// Identical on mainnet and testnet, like all genesis contracts.
    const BANK: Addr = addr!("e0b49f70991ecab05d5d7dc1f71e4ede63c8f2b7");

    const MAINNET_OWNER: Addr = addr!("149a2e2bc3ed63aeb0410416b9123d886af1f9cd");
    const TESTNET_OWNER: Addr = addr!("c4a8f7bbadd1457092a8cd182480230c0a848331");

    /// On testnet, the one account other than the DEX itself that holds LP
    /// shares (as of 2026-06-12). Its shares are deleted without
    /// compensation.
    const TESTNET_LP_HOLDER: Addr = addr!("3792b060a077fceca337334adf53f695e1362397");

    /// Raw key prefixes within the bank contract's substore, as laid out by
    /// `grug_storage`: every key component except the last is preceded by its
    /// length in 2-byte big endian.
    const BALANCE_NS: &[u8] = b"\x00\x07balance";
    const SUPPLY_NS: &[u8] = b"\x00\x06supply";
    const NAMESPACE_OWNER_NS: &[u8] = b"\x00\x0fnamespace_owner";

    fn load_fixture(filename: &str) -> BTreeMap<Binary, Binary> {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("testdata")
            .join(filename);
        let json = fs::read_to_string(&path).unwrap_or_else(|err| {
            panic!(
                "can't read fixture `{}`: {err}; see module doc for how to pull fixtures",
                path.display()
            );
        });
        serde_json::from_str(&json).unwrap()
    }

    fn seed_contract(storage: &mut dyn Storage, address: Addr, records: &BTreeMap<Binary, Binary>) {
        for (key, value) in records {
            storage.write(
                &[CONTRACT_NAMESPACE, address.as_ref(), key.as_ref()].concat(),
                value,
            );
        }
    }

    fn setup(owner: Addr, bank_fixture: &str, dex_fixture: &str) -> Shared<MockStorage> {
        let mut storage = Shared::new(MockStorage::new());

        // The migration only reads `owner` and `bank`; the other fields are
        // fillers.
        CONFIG
            .save(&mut storage, &Config {
                owner,
                bank: BANK,
                taxman: Addr::mock(0),
                cronjobs: BTreeMap::new(),
                permissions: Permissions {
                    upload: Permission::Nobody,
                    instantiate: Permission::Nobody,
                },
                max_orphan_age: Duration::from_seconds(604800),
            })
            .unwrap();

        seed_contract(&mut storage, BANK, &load_fixture(bank_fixture));
        seed_contract(&mut storage, DEX_ADDRESS, &load_fixture(dex_fixture));

        storage
    }

    /// Whether the raw denom bytes are a denom under the `dex/pool` namespace.
    fn is_lp_denom(denom: &[u8]) -> bool {
        denom == b"dex/pool" || denom.starts_with(b"dex/pool/")
    }

    fn u128_le(bytes: &[u8]) -> u128 {
        u128::from_le_bytes(bytes.try_into().unwrap())
    }

    fn balance_key(holder: Addr, denom: &str) -> Vec<u8> {
        [
            CONTRACT_NAMESPACE,
            BANK.as_ref(),
            BALANCE_NS,
            b"\x00\x14",
            holder.as_ref(),
            denom.as_bytes(),
        ]
        .concat()
    }

    fn balance_of(state: &BTreeMap<Vec<u8>, Vec<u8>>, holder: Addr, denom: &str) -> u128 {
        state
            .get(&balance_key(holder, denom))
            .map_or(0, |v| u128_le(v))
    }

    fn supply_of(state: &BTreeMap<Vec<u8>, Vec<u8>>, denom: &str) -> u128 {
        let key = [
            CONTRACT_NAMESPACE,
            BANK.as_ref(),
            SUPPLY_NS,
            denom.as_bytes(),
        ]
        .concat();
        state.get(&key).map_or(0, |v| u128_le(v))
    }

    /// The expected outcome of the migration, computed by applying the spec
    /// to the pre-migration state as a pure transformation on the raw
    /// key-value records — deliberately without using the migration's own
    /// plumbing (`grug_storage` maps, `StorageProvider`), so that the two
    /// can't share a bug.
    fn expected_post_state(
        pre: &BTreeMap<Vec<u8>, Vec<u8>>,
        owner: Addr,
    ) -> BTreeMap<Vec<u8>, Vec<u8>> {
        let bank_prefix = [CONTRACT_NAMESPACE, BANK.as_ref()].concat();
        let dex_prefix = [CONTRACT_NAMESPACE, DEX_ADDRESS.as_ref()].concat();
        let dex_namespace_owner_key = [NAMESPACE_OWNER_NS, b"dex"].concat();

        let mut post = BTreeMap::new();
        let mut swept = BTreeMap::<Vec<u8>, u128>::new();

        for (key, value) in pre {
            // 4. all records in the dex contract's storage are deleted.
            if key.starts_with(&dex_prefix) {
                continue;
            }

            if let Some(sub) = key.strip_prefix(bank_prefix.as_slice()) {
                if let Some(rest) = sub.strip_prefix(BALANCE_NS) {
                    // rest = 0x0014 ++ holder (20 bytes) ++ denom (utf-8)
                    let holder = &rest[2..22];
                    let denom = &rest[22..];

                    // 1. LP share balances are deleted for ALL holders —
                    //    checked before the dex-holder case, so LP shares are
                    //    never credited to the owner.
                    if is_lp_denom(denom) {
                        continue;
                    }

                    // 3. the dex's other balances are moved to the owner.
                    if holder == DEX_ADDRESS.as_ref() {
                        *swept.entry(denom.to_vec()).or_default() += u128_le(value);
                        continue;
                    }
                } else if let Some(denom) = sub.strip_prefix(SUPPLY_NS) {
                    // 2. LP share supplies are deleted.
                    if is_lp_denom(denom) {
                        continue;
                    }
                } else if sub == dex_namespace_owner_key.as_slice() {
                    // 5. the `dex` namespace ownership record is deleted.
                    continue;
                }
            }

            post.insert(key.clone(), value.clone());
        }

        // 3 (continued): the owner is credited additively.
        for (denom, amount) in swept {
            let key = [
                bank_prefix.as_slice(),
                BALANCE_NS,
                b"\x00\x14",
                owner.as_ref(),
                &denom,
            ]
            .concat();
            let existing = post.get(&key).map_or(0, |v| u128_le(v));
            post.insert(key, (existing + amount).to_le_bytes().to_vec());
        }

        post
    }

    /// No balance or supply record of any `dex/pool` denom may remain.
    fn assert_no_lp_records(state: &BTreeMap<Vec<u8>, Vec<u8>>) {
        let bank_prefix = [CONTRACT_NAMESPACE, BANK.as_ref()].concat();

        for key in state.keys() {
            let Some(sub) = key.strip_prefix(bank_prefix.as_slice()) else {
                continue;
            };

            if let Some(rest) = sub.strip_prefix(BALANCE_NS) {
                assert!(!is_lp_denom(&rest[22..]), "LP balance remains: {key:?}");
            } else if let Some(denom) = sub.strip_prefix(SUPPLY_NS) {
                assert!(!is_lp_denom(denom), "LP supply remains: {key:?}");
            }
        }
    }

    #[test]
    #[ignore = "requires chain state fixtures in dango/upgrade/testdata; see module doc"]
    fn cleans_up_mainnet_state() {
        let storage = setup(
            MAINNET_OWNER,
            "bank-state-mainnet.json",
            "dex-state-mainnet.json",
        );
        let pre = storage.read_with(|s| s.clone());

        // Fixture sanity: the dex holds the entire LP share supply, plus dust
        // amounts of ETH and USDC. These are frozen (the contract is dead),
        // so they hold for fixtures pulled at any time. Amounts verified
        // on-chain on 2026-06-12.
        assert_eq!(balance_of(&pre, DEX_ADDRESS, "dex/pool/eth/usdc"), 1000);
        assert_eq!(supply_of(&pre, "dex/pool/eth/usdc"), 1000);
        assert_eq!(balance_of(&pre, DEX_ADDRESS, "bridge/eth"), 423_528_684_698);
        assert_eq!(balance_of(&pre, DEX_ADDRESS, "bridge/usdc"), 316_957);

        let owner_eth_before = balance_of(&pre, MAINNET_OWNER, "bridge/eth");
        let owner_usdc_before = balance_of(&pre, MAINNET_OWNER, "bridge/usdc");

        clean_up_dex(Box::new(storage.clone())).unwrap();

        let post = storage.read_with(|s| s.clone());

        // The entire post state must equal the spec applied to the pre state.
        // This covers everything at once: LP balances and supplies deleted,
        // dust credited to the owner, namespace ownership record deleted, dex
        // substore wiped, and every other record byte-for-byte untouched.
        assert_eq!(post, expected_post_state(&pre, MAINNET_OWNER));

        // Spot checks, so a bug in `expected_post_state` can't silently
        // vacuously pass the assertion above.
        assert_no_lp_records(&post);
        assert_eq!(
            balance_of(&post, MAINNET_OWNER, "bridge/eth"),
            owner_eth_before + 423_528_684_698
        );
        assert_eq!(
            balance_of(&post, MAINNET_OWNER, "bridge/usdc"),
            owner_usdc_before + 316_957
        );
        assert_eq!(balance_of(&post, MAINNET_OWNER, "dex/pool/eth/usdc"), 0);
        assert!(
            !post
                .keys()
                .any(|k| k.starts_with(&[CONTRACT_NAMESPACE, DEX_ADDRESS.as_ref()].concat())),
            "dex contract storage must be empty"
        );

        // Running the migration a second time must change nothing.
        clean_up_dex(Box::new(storage.clone())).unwrap();
        assert_eq!(storage.read_with(|s| s.clone()), post);
    }

    #[test]
    #[ignore = "requires chain state fixtures in dango/upgrade/testdata; see module doc"]
    fn cleans_up_testnet_state() {
        let storage = setup(
            TESTNET_OWNER,
            "bank-state-testnet.json",
            "dex-state-testnet.json",
        );
        let pre = storage.read_with(|s| s.clone());

        // Fixture sanity. Unlike on mainnet, users other than the dex itself
        // hold LP shares on testnet; deleting their records without
        // compensation is accepted. The dex's own holdings (frozen; verified
        // on-chain on 2026-06-12):
        for lp_denom in [
            "dex/pool/btc/usdc",
            "dex/pool/eth/usdc",
            "dex/pool/sol/usdc",
        ] {
            assert_eq!(balance_of(&pre, DEX_ADDRESS, lp_denom), 1000);
            assert!(
                supply_of(&pre, lp_denom) > 1000,
                "others must hold LP shares"
            );
        }
        assert_eq!(
            balance_of(&pre, DEX_ADDRESS, "bridge/btc"),
            26_701_658_797_568
        );
        assert_eq!(
            balance_of(&pre, DEX_ADDRESS, "bridge/eth"),
            8_851_085_160_027_251_293_491_847
        );
        assert_eq!(
            balance_of(&pre, DEX_ADDRESS, "bridge/sol"),
            235_130_226_825_745_032
        );
        assert_eq!(balance_of(&pre, DEX_ADDRESS, "bridge/usdc"), 23_607_837);
        assert_eq!(
            balance_of(&pre, TESTNET_LP_HOLDER, "dex/pool/btc/usdc"),
            19_444_004_624_372_597
        );

        let owner_before: Vec<u128> = ["bridge/btc", "bridge/eth", "bridge/sol", "bridge/usdc"]
            .map(|denom| balance_of(&pre, TESTNET_OWNER, denom))
            .to_vec();

        clean_up_dex(Box::new(storage.clone())).unwrap();

        let post = storage.read_with(|s| s.clone());

        assert_eq!(post, expected_post_state(&pre, TESTNET_OWNER));

        // Spot checks.
        assert_no_lp_records(&post);
        assert_eq!(
            balance_of(&post, TESTNET_OWNER, "bridge/btc"),
            owner_before[0] + 26_701_658_797_568
        );
        assert_eq!(
            balance_of(&post, TESTNET_OWNER, "bridge/eth"),
            owner_before[1] + 8_851_085_160_027_251_293_491_847
        );
        assert_eq!(
            balance_of(&post, TESTNET_OWNER, "bridge/sol"),
            owner_before[2] + 235_130_226_825_745_032
        );
        assert_eq!(
            balance_of(&post, TESTNET_OWNER, "bridge/usdc"),
            owner_before[3] + 23_607_837
        );
        assert_eq!(balance_of(&post, TESTNET_LP_HOLDER, "dex/pool/btc/usdc"), 0);
        assert!(
            !post
                .keys()
                .any(|k| k.starts_with(&[CONTRACT_NAMESPACE, DEX_ADDRESS.as_ref()].concat())),
            "dex contract storage must be empty"
        );

        // Running the migration a second time must change nothing.
        clean_up_dex(Box::new(storage.clone())).unwrap();
        assert_eq!(storage.read_with(|s| s.clone()), post);
    }
}
