use {
    crate::{
        account_factory::{AccountParams, NewUserSalt, Username},
        auth::Key,
    },
    grug::{Addr, Op},
};

/// An event indicating a new user has registered.
#[grug::derive(Serde)]
#[grug::event("user_registered")]
pub struct UserRegistered {
    pub address: Addr,
    pub data: NewUserSalt,
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
