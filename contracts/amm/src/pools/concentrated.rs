use {
    dango_types::amm::{ConcentratedParams, ConcentratedPool},
    grug::{Coin, CoinPair, Uint256},
};

use super::{PoolExt, PoolInit};

impl PoolInit for ConcentratedPool {
    type Params = ConcentratedParams;

    fn initialize(_liquidity: CoinPair, _params: ConcentratedParams) -> anyhow::Result<Self> {
        todo!()
    }
}

impl PoolExt for ConcentratedPool {
    fn swap(&mut self, _input: Coin) -> anyhow::Result<(Coin, Coin)> {
        todo!()
    }

    fn provide_liquidity(&mut self, _deposit: CoinPair) -> anyhow::Result<Uint256> {
        todo!()
    }

    fn withdraw_liquidity(&mut self, _shares_to_burn: Uint256) -> anyhow::Result<CoinPair> {
        todo!()
    }
}
