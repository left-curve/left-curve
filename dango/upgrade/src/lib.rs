use {
    dango_types::account_factory::{AccountIndex, User, UserIndex, Username},
    grug::{Addr, Inner, Order, StdResult, Storage, addr},
    grug_app::{AppResult, CONTRACT_NAMESPACE, StorageProvider},
    std::collections::BTreeMap,
};

/// Address of the account factory contract.
const ACCOUNT_FACTORY: Addr = addr!("18d28bafcdf9d4574f920ea004dea2d13ec16f6b");

/// Storage layout of the account factory contract prior to this upgrade.
mod legacy {
    use {
        borsh::{BorshDeserialize, BorshSerialize},
        dango_types::{
            account_factory::{AccountIndex, UserIndex, Username},
            auth::Key,
        },
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

    pub const KEYS: Map<(UserIndex, Hash256), Key> = Map::new("key");
    pub const USER_NAMES_BY_INDEX: Map<UserIndex, Username> = Map::new("user_names__index");

    // For clearing only — key/value types don't matter.
    pub const ACCOUNTS_BY_USER: Map<u8, u8> = Map::new("account__user");
    pub const ACCOUNT_COUNT_BY_USER: Map<u8, u8> = Map::new("account_count");
    pub const USER_INDEXES_BY_NAME: Map<u8, u8> = Map::new("user_indexes__name");
}

pub fn do_upgrade<VM>(
    storage: Box<dyn Storage>,
    _vm: VM,
    _block: grug::BlockInfo,
) -> AppResult<()> {
    let mut factory_storage =
        StorageProvider::new(storage, &[CONTRACT_NAMESPACE, ACCOUNT_FACTORY.inner()]);
    let factory_storage = &mut factory_storage;

    // ======================== CODE_HASHES → CODE_HASH ========================

    // Load the Single account code hash from the old map (key 0 = AccountType::Single).
    let code_hash = legacy::CODE_HASHES.load(factory_storage, 0u8)?;

    tracing::info!(%code_hash, "Loaded Single code hash from legacy CODE_HASHES");

    // Clear the old map entries.
    legacy::CODE_HASHES.clear(factory_storage, None, None);

    // Save to the new Item-based storage.
    dango_account_factory::CODE_HASH.save(factory_storage, &code_hash)?;

    tracing::info!("Saved code hash to new CODE_HASH item");

    // ======================== Read old Accounts ==============================

    // Load all accounts with the old Borsh layout, group by owner as
    // BTreeMap<UserIndex, BTreeMap<AccountIndex, Addr>>.
    let mut accounts_by_user: BTreeMap<UserIndex, BTreeMap<AccountIndex, Addr>> = BTreeMap::new();
    for entry in legacy::ACCOUNTS.range(factory_storage, None, None, Order::Ascending) {
        let (addr, old_account) = entry?;
        let legacy::AccountParams::Single { owner } = old_account.params;
        accounts_by_user
            .entry(owner)
            .or_default()
            .insert(old_account.index, addr);
    }

    tracing::info!(
        num_users = accounts_by_user.len(),
        "Loaded accounts per user"
    );

    // ======================== Read old Keys ==================================

    let mut keys_by_user = BTreeMap::new();
    for entry in legacy::KEYS.range(factory_storage, None, None, Order::Ascending) {
        let ((user_index, key_hash), key) = entry?;
        keys_by_user
            .entry(user_index)
            .or_insert_with(BTreeMap::new)
            .insert(key_hash, key);
    }

    tracing::info!(num_users = keys_by_user.len(), "Loaded keys per user");

    // ======================== Read old usernames ==============================

    let usernames: BTreeMap<UserIndex, Username> = legacy::USER_NAMES_BY_INDEX
        .range(factory_storage, None, None, Order::Ascending)
        .collect::<StdResult<_>>()?;

    tracing::info!(num_usernames = usernames.len(), "Loaded usernames");

    // ======================== Build User structs ==============================

    let all_user_indexes: BTreeMap<UserIndex, ()> = keys_by_user
        .keys()
        .chain(accounts_by_user.keys())
        .chain(usernames.keys())
        .map(|&idx| (idx, ()))
        .collect();

    tracing::info!(num_users = all_user_indexes.len(), "Building User structs");

    for &user_index in all_user_indexes.keys() {
        let user = User {
            index: user_index,
            name: usernames
                .get(&user_index)
                .cloned()
                .unwrap_or_else(|| Username::default_for_index(user_index)),
            accounts: accounts_by_user
                .get(&user_index)
                .cloned()
                .unwrap_or_default(),
            keys: keys_by_user.get(&user_index).cloned().unwrap_or_default(),
        };

        dango_account_factory::USERS.save(factory_storage, user_index, &user)?;
    }

    tracing::info!(
        num_users = all_user_indexes.len(),
        "Saved User structs to IndexedMap"
    );

    // ======================== Clear legacy storage ============================

    legacy::CODE_HASHES.clear(factory_storage, None, None);
    legacy::ACCOUNTS.clear(factory_storage, None, None);
    legacy::KEYS.clear(factory_storage, None, None);
    legacy::ACCOUNTS_BY_USER.clear(factory_storage, None, None);
    legacy::ACCOUNT_COUNT_BY_USER.clear(factory_storage, None, None);
    legacy::USER_NAMES_BY_INDEX.clear(factory_storage, None, None);
    legacy::USER_INDEXES_BY_NAME.clear(factory_storage, None, None);

    tracing::info!("Cleared legacy storage namespaces");

    Ok(())
}
