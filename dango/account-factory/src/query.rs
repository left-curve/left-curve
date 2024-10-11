use {
    crate::{
        ACCOUNTS, ACCOUNTS_BY_USER, CODE_HASHES, DEPOSITS, KEYS, KEYS_BY_USER, NEXT_ACCOUNT_INDEX,
        USERS_BY_KEY,
    },
    dango_types::{
        account_factory::{Account, AccountIndex, AccountType, QueryMsg, User, Username},
        auth::Key,
    },
    grug::{
        Addr, Bound, Coins, Hash160, Hash256, ImmutableCtx, Json, JsonSerExt, Order, StdResult,
        Storage,
    },
    std::collections::{BTreeMap, BTreeSet},
};

const DEFAULT_PAGE_LIMIT: u32 = 30;

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> anyhow::Result<Json> {
    match msg {
        QueryMsg::NextAccountIndex {} => {
            let res = query_next_account_index(ctx.storage)?;
            res.to_json_value()
        },
        QueryMsg::CodeHash { account_type } => {
            let res = query_code_hash(ctx.storage, account_type)?;
            res.to_json_value()
        },
        QueryMsg::CodeHashes { start_after, limit } => {
            let res = query_code_hashes(ctx.storage, start_after, limit)?;
            res.to_json_value()
        },
        QueryMsg::Deposit { recipient } => {
            let res = query_deposit(ctx, recipient)?;
            res.to_json_value()
        },
        QueryMsg::Deposits { start_after, limit } => {
            let res = query_deposits(ctx, start_after, limit)?;
            res.to_json_value()
        },
        QueryMsg::Key { hash } => {
            let res = query_key(ctx.storage, hash)?;
            res.to_json_value()
        },
        QueryMsg::Keys { start_after, limit } => {
            let res = query_keys(ctx.storage, start_after, limit)?;
            res.to_json_value()
        },
        QueryMsg::KeysByUser { username } => {
            let res = query_keys_by_user(ctx.storage, &username)?;
            res.to_json_value()
        },
        QueryMsg::Account { address } => {
            let res = query_account(ctx.storage, address)?;
            res.to_json_value()
        },
        QueryMsg::Accounts { start_after, limit } => {
            let res = query_accounts(ctx.storage, start_after, limit)?;
            res.to_json_value()
        },
        QueryMsg::AccountsByUser { username } => {
            let res = query_accounts_by_user(ctx.storage, &username)?;
            res.to_json_value()
        },
        QueryMsg::User { username } => {
            let res = query_user(ctx.storage, username)?;
            res.to_json_value()
        },
        QueryMsg::UsersByKey { hash } => {
            let res = query_users_by_key(ctx.storage, hash)?;
            res.to_json_value()
        },
    }
    .map_err(Into::into)
}

fn query_next_account_index(storage: &dyn Storage) -> StdResult<AccountIndex> {
    NEXT_ACCOUNT_INDEX.current(storage)
}

fn query_code_hash(storage: &dyn Storage, account_type: AccountType) -> StdResult<Hash256> {
    CODE_HASHES.load(storage, account_type)
}

fn query_code_hashes(
    storage: &dyn Storage,
    start_after: Option<AccountType>,
    limit: Option<u32>,
) -> StdResult<BTreeMap<AccountType, Hash256>> {
    let start = start_after.map(Bound::Exclusive);
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT) as usize;

    CODE_HASHES
        .range(storage, start, None, Order::Ascending)
        .take(limit)
        .collect()
}

fn query_deposit(ctx: ImmutableCtx, recipient: Addr) -> StdResult<Option<Coins>> {
    DEPOSITS.may_load(ctx.storage, &recipient)
}

fn query_deposits(
    ctx: ImmutableCtx,
    start_after: Option<Addr>,
    limit: Option<u32>,
) -> StdResult<BTreeMap<Addr, Coins>> {
    let start = start_after.as_ref().map(Bound::Exclusive);
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT) as usize;

    DEPOSITS
        .range(ctx.storage, start, None, Order::Ascending)
        .take(limit)
        .collect()
}

fn query_key(storage: &dyn Storage, hash: Hash160) -> StdResult<Key> {
    KEYS.load(storage, hash)
}

fn query_keys(
    storage: &dyn Storage,
    start_after: Option<Hash160>,
    limit: Option<u32>,
) -> StdResult<BTreeMap<Hash160, Key>> {
    let start = start_after.map(Bound::Exclusive);
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT) as usize;

    KEYS.range(storage, start, None, Order::Ascending)
        .take(limit)
        .collect()
}

fn query_keys_by_user(
    storage: &dyn Storage,
    username: &Username,
) -> StdResult<BTreeMap<Hash160, Key>> {
    KEYS_BY_USER
        .prefix(username)
        .keys(storage, None, None, Order::Ascending)
        .map(|res| -> StdResult<_> {
            let hash = res?;
            let key = KEYS.load(storage, hash)?;
            Ok((hash, key))
        })
        .collect()
}

fn query_account(storage: &dyn Storage, address: Addr) -> StdResult<Account> {
    ACCOUNTS.load(storage, address)
}

fn query_accounts(
    storage: &dyn Storage,
    start_after: Option<Addr>,
    limit: Option<u32>,
) -> StdResult<BTreeMap<Addr, Account>> {
    let start = start_after.map(Bound::Exclusive);
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT) as usize;

    ACCOUNTS
        .range(storage, start, None, Order::Ascending)
        .take(limit)
        .collect()
}

fn query_accounts_by_user(
    storage: &dyn Storage,
    username: &Username,
) -> StdResult<BTreeMap<Addr, Account>> {
    ACCOUNTS_BY_USER
        .prefix(username)
        .keys(storage, None, None, Order::Ascending)
        .map(|res| -> StdResult<_> {
            let address = res?;
            let account = ACCOUNTS.load(storage, address)?;
            Ok((address, account))
        })
        .collect()
}

fn query_user(storage: &dyn Storage, username: Username) -> StdResult<User> {
    let keys = query_keys_by_user(storage, &username)?;
    let accounts = query_accounts_by_user(storage, &username)?;

    Ok(User { keys, accounts })
}

fn query_users_by_key(storage: &dyn Storage, hash: Hash160) -> StdResult<BTreeSet<Username>> {
    USERS_BY_KEY
        .prefix(hash)
        .keys(storage, None, None, Order::Ascending)
        .collect()
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        crate::{
            query::{query_user, query_users_by_key},
            KEYS, KEYS_BY_USER, USERS_BY_KEY,
        },
        dango_types::{account_factory::Username, auth::Key},
        grug::{btree_set, Hash160, MockStorage},
        std::{collections::BTreeSet, str::FromStr},
    };

    /// Given a key and finding all the key IDs (i.e. usernames and key hashes)
    /// associated with the key.
    ///
    /// This feature is useful if a user forgets their username, but still has
    /// the key. We can recover the username from the key.
    #[test]
    fn querying_username_by_key() {
        let mut storage = MockStorage::new();

        let u1 = Username::from_str("larry").unwrap();
        let u2 = Username::from_str("jake").unwrap();
        let u3 = Username::from_str("pumpkin").unwrap();

        let k1 = Key::Secp256r1([1; 33].into());
        let k2 = Key::Secp256k1([2; 33].into());
        let k3 = Key::Secp256k1([3; 33].into());

        let h1 = Hash160::from_inner([1; 20]);
        let h2 = Hash160::from_inner([2; 20]);
        let h3 = Hash160::from_inner([3; 20]);
        let h4 = Hash160::from_inner([4; 20]);

        // Save the following records:
        //
        // (u1, h1) => k1
        // (u1, h2) => k2
        // (u2, h1) => k1
        // (u2, h3) => k3
        //
        // Note:
        // - A username can own multiple keys
        // - A key can be owned by multiple accounts
        // - A key must have a unique hash; that is, no two different hashes can
        //   point to the same key.
        for (username, hash, key) in [
            // comment inserted to prevent undesirable rustfmt formatting
            (&u1, h1, k1),
            (&u1, h2, k2),
            (&u2, h1, k1),
            (&u2, h3, k3),
        ] {
            KEYS.save(&mut storage, hash, &key).unwrap();
            KEYS_BY_USER.insert(&mut storage, (username, hash)).unwrap();
            USERS_BY_KEY.insert(&mut storage, (hash, username)).unwrap();
        }

        // Find all usernames associated with each key hash.
        for (hash, usernames) in [
            (h1, btree_set! { u1.clone(), u2.clone() }),
            (h2, btree_set! { u1.clone() }),
            (h3, btree_set! { u2.clone() }),
            (h4, btree_set! {}),
        ] {
            let actual = query_users_by_key(&storage, hash).unwrap();
            assert_eq!(actual, usernames);
        }

        // Find all key hashes associated with each username.
        for (username, hashes) in [
            (u1, btree_set! { h1, h2 }),
            (u2, btree_set! { h1, h3 }),
            (u3, btree_set! {}),
        ] {
            let actual = query_user(&storage, username)
                .unwrap()
                .keys
                .into_keys()
                .collect::<BTreeSet<_>>();
            assert_eq!(actual, hashes);
        }
    }
}
