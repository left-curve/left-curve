use {
    dango_types::{
        account_factory::{Account, AccountIndex, AccountType, Username},
        auth::Key,
    },
    grug::{Addr, Coins, Counter, Hash256, Item, Map, Set},
};

pub const MINIMUM_DEPOSIT: Item<Coins> = Item::new("minium_deposit");

pub const CODE_HASHES: Map<AccountType, Hash256> = Map::new("hash");

pub const NEXT_ACCOUNT_INDEX: Counter<AccountIndex> = Counter::new("index", 0, 1);

pub const KEYS: Map<(&Username, Hash256), Key> = dango_auth::account_factory::KEYS;

pub const REVERSE_KEYS: Set<(Hash256, &Username)> = Set::new("reverse_key");

pub const ACCOUNTS: Map<Addr, Account> = Map::new("account");

pub const ACCOUNTS_BY_USER: Set<(&Username, Addr)> = dango_auth::account_factory::ACCOUNTS_BY_USER;
