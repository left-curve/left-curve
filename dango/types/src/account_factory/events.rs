use {
    crate::{
        account_factory::{AccountIndex, AccountParams, Username},
        auth::Key,
    },
    grug::{Addr, Hash256, Op},
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

/// An event indicating a key has been updated.
#[grug::derive(Serde)]
#[grug::event("key_updated")]
pub struct KeyUpdated {
    pub username: Username,
    pub key: Op<Key>,
}
