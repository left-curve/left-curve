use {
    crate::oracle::{PrecisionedPrice, PrecisionlessPrice, PythId},
    grug::{Addr, BorshDeExt, Map, QuerierWrapper, Storage},
};

pub const PRICES: Map<PythId, PrecisionlessPrice> = Map::new("price");

#[grug::derive(Serde, Borsh)]
pub enum PriceSource {
    Pyth {
        /// The Pyth ID of the price.
        id: PythId,
        /// The number of decimal places of the token that is used to convert
        /// the price from its smallest unit to a humanized form. E.g. 1 ATOM
        /// is 10^6 uatom, so the precision is 6.
        precision: u8,
    },
}

impl PriceSource {
    /// Directly loads the price for the price source from the storage.
    pub fn get_price(&self, storage: &dyn Storage) -> anyhow::Result<PrecisionedPrice> {
        match self {
            Self::Pyth { id, precision } => {
                let price = PRICES.load(storage, *id)?;
                Ok(price.with_precision(*precision))
            },
        }
    }

    /// Does a raw query to the oracle contract to get the price for this price source.
    pub fn raw_query_price(
        &self,
        querier: &QuerierWrapper,
        oracle: Addr,
    ) -> anyhow::Result<PrecisionedPrice> {
        match self {
            Self::Pyth { id, precision } => {
                let price = querier
                    .query_wasm_raw(oracle, PRICES.path(*id))?
                    .ok_or(anyhow::anyhow!("Price not found for denom: {}", id))?
                    .deserialize_borsh::<PrecisionlessPrice>()?
                    .with_precision(*precision);
                Ok(price)
            },
        }
    }
}
