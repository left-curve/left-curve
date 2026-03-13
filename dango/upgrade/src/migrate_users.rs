use {
    dango_types::account_factory::{AccountIndex, User, UserIndex, Username},
    grug::{Addr, Inner, Order, StdResult, Storage, addr},
    grug_app::{AppResult, CONTRACT_NAMESPACE, StorageProvider},
    std::collections::BTreeMap,
};

/// Address of the account factory contract.
const ACCOUNT_FACTORY: Addr = addr!("18d28bafcdf9d4574f920ea004dea2d13ec16f6b");

/// Old storage layout for the account factory, prior to the `User` struct
/// consolidation. We only need the maps that hold data we must read; the rest
/// are declared with dummy types so we can call `.clear()`.
mod legacy {
    use {
        dango_types::{
            account_factory::{UserIndex, Username},
            auth::Key,
        },
        grug::{Hash256, Map},
    };

    pub const KEYS: Map<(UserIndex, Hash256), Key> = Map::new("key");
    pub const USER_NAMES_BY_INDEX: Map<UserIndex, Username> = Map::new("user_names__index");

    // For clearing only — key/value types don't matter.
    pub const ACCOUNTS_BY_USER: Map<u8, u8> = Map::new("account__user");
    pub const ACCOUNT_COUNT_BY_USER: Map<u8, u8> = Map::new("account_count");
    pub const USER_INDEXES_BY_NAME: Map<u8, u8> = Map::new("user_indexes__name");
}

pub fn do_upgrade(storage: Box<dyn Storage>) -> AppResult<()> {
    let mut factory_storage =
        StorageProvider::new(storage, &[CONTRACT_NAMESPACE, ACCOUNT_FACTORY.inner()]);
    let factory_storage = &mut factory_storage;

    // 1. Read keys per user from the legacy KEYS map.
    let mut keys_by_user = BTreeMap::new();
    for entry in legacy::KEYS.range(factory_storage, None, None, Order::Ascending) {
        let ((user_index, key_hash), key) = entry?;
        keys_by_user
            .entry(user_index)
            .or_insert_with(BTreeMap::new)
            .insert(key_hash, key);
    }

    tracing::info!(num_users = keys_by_user.len(), "Loaded keys per user");

    // 2. Read accounts per user from the already-migrated ACCOUNTS map.
    //    Group by owner, sorted by account index for chronological order.
    let mut accounts_by_user: BTreeMap<UserIndex, BTreeMap<AccountIndex, Addr>> = BTreeMap::new();
    for entry in
        dango_account_factory::ACCOUNTS.range(factory_storage, None, None, Order::Ascending)
    {
        let (addr, account) = entry?;
        accounts_by_user
            .entry(account.owner)
            .or_default()
            .insert(account.index, addr);
    }

    let accounts_by_user: BTreeMap<UserIndex, Vec<Addr>> = accounts_by_user
        .into_iter()
        .map(|(idx, sorted)| (idx, sorted.into_values().collect()))
        .collect();

    tracing::info!(
        num_users = accounts_by_user.len(),
        "Loaded accounts per user"
    );

    // 3. Read usernames from legacy map.
    let usernames: BTreeMap<UserIndex, Username> = legacy::USER_NAMES_BY_INDEX
        .range(factory_storage, None, None, Order::Ascending)
        .collect::<StdResult<_>>()?;

    tracing::info!(num_usernames = usernames.len(), "Loaded usernames");

    // 4. Build the union of all user indexes.
    let all_user_indexes: BTreeMap<UserIndex, ()> = keys_by_user
        .keys()
        .chain(accounts_by_user.keys())
        .chain(usernames.keys())
        .map(|&idx| (idx, ()))
        .collect();

    tracing::info!(num_users = all_user_indexes.len(), "Building User structs");

    // 5. Build and save User structs into the new IndexedMap.
    for &user_index in all_user_indexes.keys() {
        let user = User {
            name: usernames.get(&user_index).cloned(),
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

    // 6. Clear old namespaces.
    //    Note: "user__key" is now the secondary index namespace and was already
    //    populated by `USERS.save()` above, so we do NOT clear it.
    legacy::KEYS.clear(factory_storage, None, None);
    legacy::ACCOUNTS_BY_USER.clear(factory_storage, None, None);
    legacy::ACCOUNT_COUNT_BY_USER.clear(factory_storage, None, None);
    legacy::USER_NAMES_BY_INDEX.clear(factory_storage, None, None);
    legacy::USER_INDEXES_BY_NAME.clear(factory_storage, None, None);

    tracing::info!("Cleared legacy user storage namespaces");

    Ok(())
}
