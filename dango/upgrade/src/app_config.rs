use {
    dango_types::config::{AppAddresses, AppConfig, Hyperlane},
    grug_app::AppResult,
    grug_types::{JsonDeExt, JsonSerExt, Storage},
};

/// Snapshot of the `AppConfig` schema as it existed before this upgrade.
///
/// When the spot DEX was retired we dropped three fields from the live schema:
/// `AppAddresses::dex`, `AppConfig::maker_fee_rate`, and
/// `AppConfig::taker_fee_rate`. The on-disk JSON still carries them on every
/// running chain, so this module defines the legacy shape verbatim and runs a
/// one-shot migration that re-serializes the value without the dead fields.
mod legacy_grug_app {
    use {
        grug_math::Udec128,
        grug_storage::Item,
        grug_types::{Addr, Bounded, Coins, Json, ZeroInclusiveOneExclusive},
    };

    #[grug_types::derive(Serde)]
    pub struct AppConfig {
        pub addresses: AppAddresses,
        pub minimum_deposit: Coins,
        pub maker_fee_rate: Bounded<Udec128, ZeroInclusiveOneExclusive>,
        pub taker_fee_rate: Bounded<Udec128, ZeroInclusiveOneExclusive>,
    }

    #[grug_types::derive(Serde)]
    pub struct AppAddresses {
        pub account_factory: Addr,
        pub dex: Addr,
        pub gateway: Addr,
        pub hyperlane: Hyperlane,
        pub oracle: Addr,
        pub perps: Addr,
        pub taxman: Addr,
        pub warp: Addr,
    }

    #[grug_types::derive(Serde)]
    pub struct Hyperlane {
        pub ism: Addr,
        pub mailbox: Addr,
        pub va: Addr,
    }

    /// Same storage key (`"acfg"`) as `grug_app::APP_CONFIG`, so loading
    /// through this handle reads the live on-disk bytes.
    pub const APP_CONFIG: Item<Json> = Item::new("acfg");
}

pub fn do_app_config_upgrade(storage: &mut dyn Storage) -> AppResult<()> {
    let current_app_config: legacy_grug_app::AppConfig = legacy_grug_app::APP_CONFIG
        .load(storage)?
        .deserialize_json()?;

    let new_app_config = AppConfig {
        addresses: AppAddresses {
            account_factory: current_app_config.addresses.account_factory,
            gateway: current_app_config.addresses.gateway,
            hyperlane: Hyperlane {
                ism: current_app_config.addresses.hyperlane.ism,
                mailbox: current_app_config.addresses.hyperlane.mailbox,
                va: current_app_config.addresses.hyperlane.va,
            },
            oracle: current_app_config.addresses.oracle,
            perps: current_app_config.addresses.perps,
            taxman: current_app_config.addresses.taxman,
            warp: current_app_config.addresses.warp,
        },
        minimum_deposit: current_app_config.minimum_deposit,
    };

    grug_app::APP_CONFIG.save(storage, &new_app_config.to_json_value()?)?;

    tracing::info!("Migrated AppConfig: dropped `dex`, `maker_fee_rate`, `taker_fee_rate`");

    Ok(())
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        grug_types::{Inner, Json, MockStorage, json},
    };

    /// Verbatim snapshot of the live mainnet `AppConfig`, retrieved via
    /// `queryApp(request: { app_config: {} })` against
    /// `https://api-mainnet.dango.zone/graphql`. Using the real production
    /// value is the strongest signal that the migration handles the exact
    /// shape the hard fork will run against in production.
    fn mainnet_app_config_json() -> Json {
        json!({
            "addresses": {
                "account_factory": "0x18d28bafcdf9d4574f920ea004dea2d13ec16f6b",
                "dex": "0xda32476efe31e535207f0ad690d337a4ebf54a22",
                "gateway": "0xc51e2cbe9636a90c86463ac3eb18fbee92b700d1",
                "hyperlane": {
                    "ism": "0xdc68d6c82f5e4386294e7fda27317ab6ae8ff54c",
                    "mailbox": "0x974e57564ed3ed7d8f99d0c359fd03f3d78259c7",
                    "va": "0x75f38c6fcfc2fb8333e5c3ef89d13b7036abe3ff"
                },
                "oracle": "0xcedc5f73cbb963a48471b849c3650e6e34cd3b6d",
                "perps": "0x90bc84df68d1aa59a857e04ed529e9a26edbea4f",
                "taxman": "0xda70a9c1417aee00f960fe896add9d571f9c365b",
                "warp": "0x981e6817442143ce5128992c7ab4a317321f00e9"
            },
            "maker_fee_rate": "0.0002",
            "minimum_deposit": {
                "bridge/eth": "3000000000000000",
                "bridge/usdc": "10000000"
            },
            "taker_fee_rate": "0.0005"
        })
    }

    #[test]
    fn drops_dead_fields_and_preserves_the_rest() {
        let mut storage = MockStorage::new();

        // Seed the chain's pre-upgrade state with the live mainnet JSON.
        legacy_grug_app::APP_CONFIG
            .save(&mut storage, &mainnet_app_config_json())
            .unwrap();

        do_app_config_upgrade(&mut storage).unwrap();

        // The post-upgrade value must deserialize cleanly into the new
        // `AppConfig` struct, and every surviving field must equal what we
        // started with.
        let migrated: AppConfig = grug_app::APP_CONFIG
            .load(&storage)
            .unwrap()
            .deserialize_json()
            .unwrap();

        let original: legacy_grug_app::AppConfig =
            mainnet_app_config_json().deserialize_json().unwrap();

        assert_eq!(
            migrated.addresses.account_factory,
            original.addresses.account_factory,
        );
        assert_eq!(migrated.addresses.gateway, original.addresses.gateway);
        assert_eq!(
            migrated.addresses.hyperlane.ism,
            original.addresses.hyperlane.ism,
        );
        assert_eq!(
            migrated.addresses.hyperlane.mailbox,
            original.addresses.hyperlane.mailbox,
        );
        assert_eq!(
            migrated.addresses.hyperlane.va,
            original.addresses.hyperlane.va,
        );
        assert_eq!(migrated.addresses.oracle, original.addresses.oracle);
        assert_eq!(migrated.addresses.perps, original.addresses.perps);
        assert_eq!(migrated.addresses.taxman, original.addresses.taxman);
        assert_eq!(migrated.addresses.warp, original.addresses.warp);
        assert_eq!(migrated.minimum_deposit, original.minimum_deposit);

        // The raw JSON written back must NOT contain any of the dead keys —
        // not at the top level (`maker_fee_rate`, `taker_fee_rate`) nor
        // nested under `addresses` (`dex`).
        let written = grug_app::APP_CONFIG.load(&storage).unwrap().into_inner();
        let object = written
            .as_object()
            .expect("app_config must be a JSON object");

        assert!(!object.contains_key("maker_fee_rate"));
        assert!(!object.contains_key("taker_fee_rate"));
        assert!(!object["addresses"].as_object().unwrap().contains_key("dex"),);
    }
}
