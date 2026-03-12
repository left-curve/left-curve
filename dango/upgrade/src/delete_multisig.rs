use {
    grug::{Addr, Inner, Order, StdResult, Storage, addr},
    grug_app::{AppResult, CONTRACT_NAMESPACE, StorageProvider},
};

/// Address of the account factory contract.
const ACCOUNT_FACTORY: Addr = addr!("18d28bafcdf9d4574f920ea004dea2d13ec16f6b");

/// Storage layout of the account factory contract prior to this upgrade.
///
/// Before this upgrade, `CODE_HASHES` was a `Map<AccountType, Hash256>` keyed
/// by account type (Single = 0, Multi = 1). The `PrimaryKey` impl for
/// `AccountType` produces `RawKey::Fixed8([index])`, which is identical to the
/// `PrimaryKey` impl for `u8`. We can therefore use `Map<u8, Hash256>` to read
/// the old entries.
///
/// Similarly, `Account` previously had an `AccountParams` enum wrapper around
/// the owner field. We define a legacy Borsh-compatible struct to deserialize
/// the old layout.
mod legacy_account_factory {
    use {
        borsh::{BorshDeserialize, BorshSerialize},
        dango_types::account_factory::AccountIndex,
        grug::{Addr, Hash256, Map},
    };

    pub const CODE_HASHES: Map<u8, Hash256> = Map::new("hash");

    pub const ACCOUNTS: Map<Addr, Account> = Map::new("account");

    /// Old `Account` Borsh layout: `u32` index + `u8` enum discriminant + `u32` owner.
    #[derive(BorshSerialize, BorshDeserialize)]
    pub struct Account {
        pub index: AccountIndex,
        pub params: AccountParams,
    }

    #[derive(BorshSerialize, BorshDeserialize)]
    pub enum AccountParams {
        Single { owner: u32 },
    }
}

pub fn do_upgrade(storage: Box<dyn Storage>) -> AppResult<()> {
    let mut factory_storage =
        StorageProvider::new(storage, &[CONTRACT_NAMESPACE, ACCOUNT_FACTORY.inner()]);
    let factory_storage = &mut factory_storage;

    // ------------------------ CODE_HASHES → CODE_HASH ------------------------

    // Load the Single account code hash from the old map (key 0 = AccountType::Single).
    let code_hash = legacy_account_factory::CODE_HASHES.load(factory_storage, 0u8)?;

    tracing::info!(%code_hash, "Loaded Single code hash from legacy CODE_HASHES");

    // Clear the old map entries.
    legacy_account_factory::CODE_HASHES.clear(factory_storage, None, None);

    tracing::info!("Cleared legacy CODE_HASHES map");

    // Save to the new Item-based storage.
    dango_account_factory::CODE_HASH.save(factory_storage, &code_hash)?;

    tracing::info!("Saved code hash to new CODE_HASH item");

    // -------------------------- ACCOUNTS migration ---------------------------

    // Load all accounts with the old Borsh layout.
    let old_accounts = legacy_account_factory::ACCOUNTS
        .range(factory_storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()?;

    tracing::info!(
        num_accounts = old_accounts.len(),
        "Loaded accounts with legacy layout"
    );

    // Re-save each account with the new layout (no enum wrapper).
    for (addr, old_account) in &old_accounts {
        let legacy_account_factory::AccountParams::Single { owner } = old_account.params;

        let new_account = dango_types::account_factory::Account {
            index: old_account.index,
            owner,
        };

        dango_account_factory::ACCOUNTS.save(factory_storage, *addr, &new_account)?;
    }

    tracing::info!(
        num_accounts = old_accounts.len(),
        "Migrated accounts to new layout"
    );

    tracing::info!("Account factory migration completed");

    Ok(())
}
