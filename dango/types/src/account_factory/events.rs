use {
    super::{AccountParams, Username},
    crate::auth::Key,
    grug::{Addr, Op},
};

/// An event indicating a new user has registered.
#[grug::derive(Serde)]
#[grug::event("user_registered")]
pub struct UserRegistered {
    pub username: Username,
    pub address: Addr,
    pub key: Key,
}

/// An event indicating a new address has been created.
#[grug::derive(Serde)]
#[grug::event("account_registered")]
pub struct AccountRegistered {
    pub address: Addr,
    pub params: AccountParams,
}

/// An event indicating a key has been updated.
#[grug::derive(Serde)]
#[grug::event("key_updated")]
pub struct KeyUpdated {
    pub username: Username,
    pub key: Op<Key>,
}
