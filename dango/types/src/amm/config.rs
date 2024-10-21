use grug::{Bound, Bounded, Bounds, Coin, NonZero, NumberConst, Udec128};

/// Defines the bounds for a fee rate: 0 <= FeeRate < 1.
#[grug::derive(Serde)]
pub struct FeeRateBounds;

impl Bounds<Udec128> for FeeRateBounds {
    const MAX: Option<Bound<Udec128>> = Some(Bound::Exclusive(Udec128::ONE));
    const MIN: Option<Bound<Udec128>> = None;
}

/// A decimal bounded by the fee rate bounds.
pub type FeeRate = Bounded<Udec128, FeeRateBounds>;

/// Global configuration of the AMM.
#[grug::derive(Serde, Borsh)]
pub struct Config {
    /// The amount of fee that must be paid in order to create a pool.
    ///
    /// For reference, on Osmosis this is currently set to 20 OSMO or ~$10,
    /// which can be queried by:
    ///
    /// ```sh
    /// osmosisd q poolmanager params --node https://rpc.osmosis.zone:443
    /// ```
    pub pool_creation_fee: NonZero<Coin>,
    /// Percentage of the final swap output that is charged as protocol fee,
    /// paid to token stakers.
    ///
    /// Note to be confused with the liquidity fee, which is configured on a
    /// per-pool basis, and paid to liquidity providers.
    pub protocol_fee_rate: FeeRate,
}
