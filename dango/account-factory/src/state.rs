use {
    dango_types::account_factory::{Account, AccountIndex, User, UserIndex, Username},
    grug::{Addr, Counter, Hash256, Item, Map, Set},
};

pub const CODE_HASH: Item<Hash256> = Item::new("hash");

pub const NEXT_USER_INDEX: Counter<UserIndex> = Counter::new("user_index", 0, 1);

pub const NEXT_ACCOUNT_INDEX: Counter<AccountIndex> = Counter::new("account_index", 0, 1);

// TODO: Convert this to an `IndexedMap` that supports looking up user profiles
// by username or key hash. (The `USERS_BY_KEY`, `USERS_BY_NAME` maps can then be deleted.)
// Current available index types don't support this, because a user may have
// multiple keys, or no username. Both `UniqueIndex` and `MultiIndex` expects
// exactly one value to be indexed.
pub const USERS: Map<UserIndex, User> = dango_auth::account_factory::USERS;

pub const USERS_BY_KEY: Set<(Hash256, UserIndex)> = Set::new("user__key");

pub const USER_INDEXES_BY_NAME: Map<&Username, UserIndex> = Map::new("user_indexes__name");

pub const ACCOUNTS: Map<Addr, Account> = Map::new("account");
