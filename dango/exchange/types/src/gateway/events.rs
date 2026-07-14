use {
    super::{Addr32, Remote},
    dango_math::Uint128,
    dango_primitives::{Addr, Denom},
};

/// Event indicating tokens have been received from a remote chain and
/// credited to a Dango account (a deposit).
#[dango_primitives::derive(Serde)]
#[dango_primitives::event("deposited")]
pub struct Deposited {
    /// The Dango account that received the tokens.
    pub user: Addr,
    /// The bridge contract that relayed the transfer.
    pub bridge: Addr,
    /// The remote chain the tokens originated from.
    pub remote: Remote,
    pub denom: Denom,
    pub amount: Uint128,
}

/// Event indicating tokens have been sent to a remote chain (a withdrawal).
#[dango_primitives::derive(Serde)]
#[dango_primitives::event("withdrawn")]
pub struct Withdrawn {
    /// The Dango account that initiated the withdrawal.
    pub user: Addr,
    /// The bridge contract that handles the transfer.
    pub bridge: Addr,
    /// The remote chain the tokens are sent to.
    pub remote: Remote,
    /// The recipient address on the remote chain.
    pub recipient: Addr32,
    pub denom: Denom,
    /// The amount bridged to the remote chain, after deducting the
    /// withdrawal fee.
    pub amount: Uint128,
    /// The withdrawal fee charged; zero if no fee is configured for the
    /// route.
    pub fee: Uint128,
}
