use {
    crate::oracle::{PrecisionedPrice, PrecisionlessPrice, PythId},
    grug::{Map, Storage},
};

pub const PRICES: Map<PythId, PrecisionlessPrice> = Map::new("price");

#[grug::derive(Serde, Borsh)]
pub enum PriceSource {
    Pyth { id: PythId, precision: u8 },
}

impl PriceSource {
    pub fn get_price(&self, storage: &dyn Storage) -> anyhow::Result<PrecisionedPrice> {
        match self {
            Self::Pyth { id, precision } => {
                let price = PRICES.load(storage, *id)?;
                Ok(price.with_precision(*precision))
            },
        }
    }
}
