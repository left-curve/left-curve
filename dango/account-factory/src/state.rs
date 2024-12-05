use {
    dango_types::{
        account_factory::{Account, AccountIndex, AccountType, Username},
        auth::Key,
    },
    grug::{Addr, Coins, Counter, Hash160, Hash256, Map, Set},
};

pub const CODE_HASHES: Map<AccountType, Hash256> = Map::new("hash");

pub const NEXT_ACCOUNT_INDEX: Counter<AccountIndex> = Counter::new("index", 0, 1);

pub const DEPOSITS: Map<&Addr, Coins> = Map::new("deposit");

pub const KEYS: Map<(&Username, Hash160), Key> = Map::new("key");

pub const ACCOUNTS: Map<Addr, Account> = Map::new("account");

pub const ACCOUNTS_BY_USER: Set<(&Username, Addr)> = Set::new("account__user");
