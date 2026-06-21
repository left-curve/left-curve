use {
    dango_app::{APP_CONFIG, AppResult, CONFIG, CONTRACT_NAMESPACE, StorageProvider},
    dango_primitives::{Config, JsonDeExt, Storage, btree_set},
    dango_storage::Item,
    dango_types::config::AppConfig,
};

/// Storage shapes as they existed before the taxman contract was removed in
/// 0.26.0.
mod legacy {
    use {
        borsh::{BorshDeserialize, BorshSerialize},
        dango_math::Udec128,
        dango_primitives::{Addr, Denom, Duration, Permissions},
        std::collections::BTreeMap,
    };

    /// The chain `Config`, before the `taxman` field was replaced by the
    /// `gas_token`, `gas_fee_rate`, and `gas_exemptions` fields.
    #[derive(BorshSerialize, BorshDeserialize)]
    pub struct Config {
        pub owner: Addr,
        pub bank: Addr,
        pub taxman: Addr,
        pub cronjobs: BTreeMap<Addr, Duration>,
        pub permissions: Permissions,
        pub max_orphan_age: Duration,
    }

    /// The taxman contract's stored fee configuration.
    #[derive(BorshSerialize, BorshDeserialize)]
    pub struct TaxmanConfig {
        pub fee_denom: Denom,
        pub fee_rate: Udec128,
    }
}

/// The chain config is stored under the `cnfg` namespace (see `dango_app`'s
/// `CONFIG`).
const LEGACY_CONFIG: Item<legacy::Config> = Item::new("cnfg");

/// The taxman contract stored its fee config under the `config` key in its own
/// substore.
const LEGACY_TAXMAN_CONFIG: Item<legacy::TaxmanConfig> = Item::new("config");

/// Migrate the chain config from the taxman-based gas model to the inlined one.
///
/// The `taxman` field is dropped; `gas_token` and `gas_fee_rate` are populated
/// from the taxman contract's stored config; and `gas_exemptions` is populated
/// with the oracle and account-factory addresses (read from the app config),
/// which send protocol-level transactions and must remain fee-exempt.
pub fn do_taxman_removal_upgrade(storage: Box<dyn Storage>) -> AppResult<()> {
    // Load the pre-upgrade chain config and app config.
    let legacy_cfg = LEGACY_CONFIG.load(&storage)?;
    let app_cfg = APP_CONFIG.load(&storage)?.deserialize_json::<AppConfig>()?;

    // Read the taxman contract's fee config from its substore, then recover the
    // base storage.
    let taxman_storage = StorageProvider::new(storage, &[CONTRACT_NAMESPACE, &legacy_cfg.taxman]);
    let taxman_cfg = LEGACY_TAXMAN_CONFIG.load(&taxman_storage)?;
    let mut storage = taxman_storage.into_inner();

    // Build and save the new chain config.
    let new_cfg = Config {
        owner: legacy_cfg.owner,
        bank: legacy_cfg.bank,
        gas_token: taxman_cfg.fee_denom,
        gas_fee_rate: taxman_cfg.fee_rate,
        gas_exemptions: btree_set! {
            app_cfg.addresses.account_factory,
            app_cfg.addresses.oracle,
        },
        cronjobs: legacy_cfg.cronjobs,
        permissions: legacy_cfg.permissions,
        max_orphan_age: legacy_cfg.max_orphan_age,
    };

    CONFIG.save(&mut storage, &new_cfg)?;

    Ok(())
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        dango_math::Udec128,
        dango_primitives::{
            Addr, Coins, Duration, JsonSerExt, MockStorage, Permission, Permissions, Shared,
            btree_map,
        },
        dango_types::config::{AppAddresses, Hyperlane},
        std::str::FromStr,
    };

    #[test]
    fn taxman_removal_upgrade() {
        let owner = Addr::mock(1);
        let bank = Addr::mock(2);
        let taxman = Addr::mock(3);
        let account_factory = Addr::mock(4);
        let oracle = Addr::mock(5);
        let gateway = Addr::mock(6);

        let fee_denom = "bridge/usdc".parse().unwrap();
        let fee_rate = Udec128::from_str("0.25").unwrap();

        // A shared handle so we can inspect the storage after the upgrade
        // (which consumes the `Box` handed to it).
        let storage = Shared::new(MockStorage::new());

        // Seed the pre-upgrade chain config.
        LEGACY_CONFIG
            .save(&mut storage.clone(), &legacy::Config {
                owner,
                bank,
                taxman,
                cronjobs: btree_map! { gateway => Duration::from_minutes(1) },
                permissions: Permissions {
                    upload: Permission::Nobody,
                    instantiate: Permission::Everybody,
                },
                max_orphan_age: Duration::from_weeks(1),
            })
            .unwrap();

        // Seed the app config. Only the addresses matter here; the stale
        // `taxman` address (still present pre-upgrade) is intentionally included
        // to exercise that it is tolerated.
        let app_config = AppConfig {
            addresses: AppAddresses {
                account_factory,
                gateway,
                hyperlane: Hyperlane {
                    ism: Addr::mock(7),
                    mailbox: Addr::mock(8),
                    va: Addr::mock(9),
                },
                oracle,
                perps: Addr::mock(10),
                warp: Addr::mock(11),
            },
            minimum_deposit: Coins::new(),
        };
        APP_CONFIG
            .save(&mut storage.clone(), &app_config.to_json_value().unwrap())
            .unwrap();

        // Seed the taxman contract's fee config into its substore.
        let mut taxman_storage =
            StorageProvider::new(Box::new(storage.clone()), &[CONTRACT_NAMESPACE, &taxman]);
        LEGACY_TAXMAN_CONFIG
            .save(&mut taxman_storage, &legacy::TaxmanConfig {
                fee_denom: "bridge/usdc".parse().unwrap(),
                fee_rate,
            })
            .unwrap();

        // Run the upgrade.
        do_taxman_removal_upgrade(Box::new(storage.clone())).unwrap();

        // The new chain config must carry over the unchanged fields, adopt the
        // gas params from the taxman, and exempt the oracle and account factory.
        let new_cfg = CONFIG.load(&storage).unwrap();
        assert_eq!(new_cfg.owner, owner);
        assert_eq!(new_cfg.bank, bank);
        assert_eq!(new_cfg.gas_token, fee_denom);
        assert_eq!(new_cfg.gas_fee_rate, fee_rate);
        assert_eq!(
            new_cfg.gas_exemptions,
            btree_set! { account_factory, oracle }
        );
        assert_eq!(new_cfg.cronjobs, btree_map! {
            gateway => Duration::from_minutes(1)
        });
        assert_eq!(new_cfg.max_orphan_age, Duration::from_weeks(1));
    }
}
