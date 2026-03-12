use {
    crate::{
        account_factory::{AccountIndex, UserIndex},
        auth::Key,
    },
    grug::{Addr, Hash256},
};

/// An event indicating a new user has registered.
#[grug::derive(Serde)]
#[grug::event("user_registered")]
pub struct UserRegistered {
    pub user_index: UserIndex,
    pub key_hash: Hash256,
    pub key: Key,
}

/// An event indicating a new address has been created.
#[grug::derive(Serde)]
#[grug::event("account_registered")]
pub struct AccountRegistered {
    pub account_index: AccountIndex,
    pub address: Addr,
    pub owner: UserIndex,
}

/// An event indicating a username begins to own an account.
#[grug::derive(Serde)]
#[grug::event("account_owned")]
pub struct AccountOwned {
    pub user_index: UserIndex,
    pub address: Addr,
}

/// An event indicating a username begins to own a key.
#[grug::derive(Serde)]
#[grug::event("key_owned")]
pub struct KeyOwned {
    pub user_index: UserIndex,
    pub key_hash: Hash256,
    pub key: Key,
}

/// An event indicating a username ceases to own a key.
#[grug::derive(Serde)]
#[grug::event("key_disowned")]
pub struct KeyDisowned {
    pub user_index: UserIndex,
    pub key_hash: Hash256,
}
