use {
    crate::{
        ACCOUNTS, ACCOUNTS_BY_USER, CODE_HASHES, KEYS, MINIMUM_DEPOSIT, NEXT_ACCOUNT_INDEX,
        REVERSE_KEYS,
    },
    dango_types::{
        account_factory::{
            Account, AccountIndex, AccountType, QueryKeyPaginateParam, QueryKeyResponseItem,
            QueryMsg, User, Username,
        },
        auth::Key,
    },
    grug::{
        Addr, Bound, Coins, DEFAULT_PAGE_LIMIT, Hash256, ImmutableCtx, Json, JsonSerExt, Order,
        StdResult, Storage,
    },
    std::collections::{BTreeMap, BTreeSet},
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> anyhow::Result<Json> {
    match msg {
        QueryMsg::MinimumDeposit {} => {
            let res = query_minimum_deposit(ctx.storage)?;
            res.to_json_value()
        },
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
        QueryMsg::Key { hash, username } => {
            let res = query_key(ctx.storage, hash, username)?;
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
        QueryMsg::UsersByKey {
            key_hash,
            start_after,
            limit,
        } => {
            let res = query_users_by_key(ctx.storage, key_hash, start_after, limit)?;
            res.to_json_value()
        },
    }
    .map_err(Into::into)
}

fn query_minimum_deposit(storage: &dyn Storage) -> StdResult<Coins> {
    MINIMUM_DEPOSIT.load(storage)
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

fn query_key(storage: &dyn Storage, hash: Hash256, username: Username) -> StdResult<Key> {
    KEYS.load(storage, (&username, hash))
}

fn query_keys(
    storage: &dyn Storage,
    start_after: Option<QueryKeyPaginateParam>,
    limit: Option<u32>,
) -> StdResult<Vec<QueryKeyResponseItem>> {
    let start = start_after
        .as_ref()
        .map(|param| Bound::Exclusive((&param.username, param.key_hash)));
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT) as usize;

    KEYS.range(storage, start, None, Order::Ascending)
        .take(limit)
        .map(|res| {
            let ((username, key_hash), key) = res?;
            Ok(QueryKeyResponseItem {
                username,
                key_hash,
                key,
            })
        })
        .collect()
}

fn query_keys_by_user(
    storage: &dyn Storage,
    username: &Username,
) -> StdResult<BTreeMap<Hash256, Key>> {
    KEYS.prefix(username)
        .range(storage, None, None, Order::Ascending)
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

fn query_users_by_key(
    storage: &dyn Storage,
    key_hash: Hash256,
    start_after: Option<Username>,
    limit: Option<u32>,
) -> StdResult<BTreeSet<Username>> {
    let start = start_after.as_ref().map(Bound::Exclusive);
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT) as usize;

    REVERSE_KEYS
        .prefix(key_hash)
        .keys(storage, start, None, Order::Ascending)
        .take(limit)
        .collect()
}
