use {
    crate::{ACCOUNTS, CODE_HASH, NEXT_ACCOUNT_INDEX, NEXT_USER_INDEX, USERS},
    dango_types::account_factory::{
        Account, AccountIndex, QueryMsg, User, UserIndex, UserIndexAndName, UserIndexOrName,
    },
    grug::{
        Addr, Bound, DEFAULT_PAGE_LIMIT, Hash256, ImmutableCtx, Json, JsonSerExt, Order, StdResult,
        Storage,
    },
    std::collections::BTreeMap,
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> anyhow::Result<Json> {
    match msg {
        QueryMsg::CodeHash {} => {
            let res = query_code_hash(ctx.storage)?;
            res.to_json_value()
        },
        QueryMsg::NextUserIndex {} => {
            let res = query_next_user_index(ctx.storage)?;
            res.to_json_value()
        },
        QueryMsg::NextAccountIndex {} => {
            let res = query_next_account_index(ctx.storage)?;
            res.to_json_value()
        },
        QueryMsg::User(index_or_name) => {
            let res = query_user(ctx.storage, index_or_name)?;
            res.to_json_value()
        },
        QueryMsg::Users { start_after, limit } => {
            let res = query_users(ctx.storage, start_after, limit)?;
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
        QueryMsg::ForgotUsername {
            key_hash,
            start_after,
            limit,
        } => {
            let res = forgot_username(ctx.storage, key_hash, start_after, limit)?;
            res.to_json_value()
        },
    }
    .map_err(Into::into)
}

fn query_code_hash(storage: &dyn Storage) -> StdResult<Hash256> {
    CODE_HASH.load(storage)
}

fn query_next_user_index(storage: &dyn Storage) -> StdResult<UserIndex> {
    NEXT_USER_INDEX.current(storage)
}

fn query_next_account_index(storage: &dyn Storage) -> StdResult<AccountIndex> {
    NEXT_ACCOUNT_INDEX.current(storage)
}

fn query_user(storage: &dyn Storage, index_or_name: UserIndexOrName) -> StdResult<User> {
    match index_or_name {
        UserIndexOrName::Index(idx) => USERS.load(storage, idx),
        UserIndexOrName::Name(username) => USERS.idx.by_name.load_value(storage, username),
    }
}

fn query_users(
    storage: &dyn Storage,
    start_after: Option<UserIndex>,
    limit: Option<u32>,
) -> StdResult<BTreeMap<UserIndex, User>> {
    let start = start_after.map(Bound::Exclusive);
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT) as usize;

    USERS
        .range(storage, start, None, Order::Ascending)
        .take(limit)
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

fn forgot_username(
    storage: &dyn Storage,
    key_hash: Hash256,
    start_after: Option<UserIndex>,
    limit: Option<u32>,
) -> StdResult<Vec<UserIndexAndName>> {
    let start = start_after.map(Bound::Exclusive);
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT) as usize;

    USERS
        .idx
        .by_key
        .prefix(key_hash)
        .range(storage, start, None, Order::Ascending)
        .map(|res| {
            let (index, user) = res?;
            Ok(UserIndexAndName {
                index,
                name: user.name,
            })
        })
        .take(limit)
        .collect()
}
