use grug::{Coin, CoinPair, Uint256};

// Note: this trait is not object-safe, because of:
// - it has an associated type;
// - it has `Sized` as a super-trait.
// Therefore, we split it off from `PoolExt` which is intended as an object-safe trait.
pub trait PoolInit: Sized {
    type Params;

    fn initialize(liquidity: CoinPair, params: Self::Params) -> anyhow::Result<Self>;
}

pub trait PoolExt {
    /// Perform a swap operation.
    ///
    /// Returns:
    /// 1. swap output;
    /// 2. liquidity fee charged.
    ///
    /// We don't actually use the liquidity fee amount in contract logics.
    /// We just output it in events for data logging purpose.
    fn swap(&mut self, input: Coin) -> anyhow::Result<(Coin, Coin)>;

    /// Provide liquidity to the pool.
    /// Returns the amount of liquidity tokens to be minted.
    fn provide_liquidity(&mut self, deposit: CoinPair) -> anyhow::Result<Uint256>;

    /// Withdraw liquidity from the pool.
    /// Returns the amount of liquidity to be refunded to the user.
    fn withdraw_liquidity(&mut self, shares_to_burn: Uint256) -> anyhow::Result<CoinPair>;
}
