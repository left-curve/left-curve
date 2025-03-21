use {
    super::{AccountParams, Username},
    grug::Addr,
};

/// An event indicating a new user has registered.
#[grug::derive(Serde)]
#[grug::event("user_registered")]
pub struct UserRegistered {
    pub username: Username,
    pub address: Addr,
}

/// An event indicating a new address has been created.
#[grug::derive(Serde)]
#[grug::event("account_registered")]
pub struct AccountRegistered {
    pub address: Addr,
    pub params: AccountParams,
}
