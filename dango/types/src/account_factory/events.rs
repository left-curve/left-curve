use {
    crate::{
        account_factory::{AccountIndex, AccountParams, Username},
        auth::Key,
    },
    grug::{Addr, Hash256},
};

/// An event indicating a new user has registered.
#[grug::derive(Serde)]
#[grug::event("user_registered")]
pub struct UserRegistered {
    pub username: Username,
    pub key: Key,
    pub key_hash: Hash256,
}

/// An event indicating a new address has been created.
#[grug::derive(Serde)]
#[grug::event("account_registered")]
pub struct AccountRegistered {
    pub address: Addr,
    pub params: AccountParams,
    pub index: AccountIndex,
}

/// An event indicating a username begin to own an account.
#[grug::derive(Serde)]
#[grug::event("account_owned")]
pub struct AccountOwned {
    pub username: Username,
    pub address: Addr,
}

/// An event indicating a username cease to own an account.
#[grug::derive(Serde)]
#[grug::event("account_disowned")]
pub struct AccountDisowned {
    pub username: Username,
    pub address: Addr,
}

/// An event indicating a username begin to own a key.
#[grug::derive(Serde)]
#[grug::event("key_owned")]
pub struct KeyOwned {
    pub username: Username,
    pub key_hash: Hash256,
    pub key: Key,
}

/// An event indicating a username cease to own a key.
#[grug::derive(Serde)]
#[grug::event("key_disowned")]
pub struct KeyDisowned {
    pub username: Username,
    pub key_hash: Hash256,
}
