use dango_primitives::{Addr, Coins};

/// An event indicating a user has sent a transfer of coins.
#[dango_primitives::derive(Serde)]
#[dango_primitives::event("sent")]
pub struct Sent {
    pub user: Addr,
    pub to: Addr,
    pub coins: Coins,
}

/// An event indicating a user has received a transfer of coins.
#[dango_primitives::derive(Serde)]
#[dango_primitives::event("received")]
pub struct Received {
    pub user: Addr,
    pub from: Addr,
    pub coins: Coins,
}

/// An event indicating a user has received newly minted coins.
#[dango_primitives::derive(Serde)]
#[dango_primitives::event("minted")]
pub struct Minted {
    pub user: Addr,
    /// The account that initiated the minting.
    ///
    /// E.g. `user` is providing liquidity to the Dango DEX contract. In this
    /// case, the DEX contract will initiate the minting of LP tokens to `user`'s
    /// wallet.
    pub minter: Addr,
    pub coins: Coins,
}

/// An event indicating a user's coins have been burned from his wallet.
#[dango_primitives::derive(Serde)]
#[dango_primitives::event("burned")]
pub struct Burned {
    pub user: Addr,
    pub burner: Addr,
    pub coins: Coins,
}

/// An event indicating a transfer has been attempted of which the recipient
/// account doesn't exist (i.e. an "orphaned transfer").
///
/// The funds are temporarily held in the Dango bank contract, and can be
/// claimed either by the sender or the recipient (once it's been created).
#[dango_primitives::derive(Serde)]
#[dango_primitives::event("transfer_orphaned")]
pub struct TransferOrphaned {
    pub from: Addr,
    pub to: Addr,
    pub coins: Coins,
}
