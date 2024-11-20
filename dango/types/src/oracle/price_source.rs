use {
    crate::oracle::{PrecisionedPrice, PrecisionlessPrice, PythId},
    grug::{Map, Storage},
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
}
