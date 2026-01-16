use {
    dango_types::{
        account_factory::{Account, AccountIndex, AccountType, UserIndex, Username},
        auth::Key,
    },
    grug::{Addr, Counter, Counters, Hash256, Map, Set},
};

pub const CODE_HASHES: Map<AccountType, Hash256> = Map::new("hash");

pub const NEXT_USER_INDEX: Counter<UserIndex> = Counter::new("user_index", 0, 1);

pub const NEXT_ACCOUNT_INDEX: Counter<AccountIndex> = Counter::new("account_index", 0, 1);

pub const KEYS: Map<(UserIndex, Hash256), Key> = dango_auth::account_factory::KEYS;

pub const USERS_BY_KEY: Set<(Hash256, UserIndex)> = Set::new("user__key");

pub const ACCOUNTS: Map<Addr, Account> = Map::new("account");

pub const ACCOUNTS_BY_USER: Set<(UserIndex, Addr)> = dango_auth::account_factory::ACCOUNTS_BY_USER;

// Base is default to 1, because an account is opened automatically upon the
// creation of each user.
pub const ACCOUNT_COUNT_BY_USER: Counters<UserIndex, u8> = Counters::new("account_count", 1, 1);

pub const USER_NAMES_BY_INDEX: Map<UserIndex, Username> = Map::new("user_names__index");

pub const USER_INDEXES_BY_NAME: Map<&Username, UserIndex> = Map::new("user_indexes__name");
