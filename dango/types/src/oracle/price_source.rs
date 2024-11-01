use {
    super::{PrecisionedPrice, PrecisionlessPrice, PythId},
    grug::{Map, Storage},
    std::ops::Deref,
};

#[grug::derive(Serde, Borsh)]
pub enum PriceSourceCollector {
    Pyth(PythPriceSource),
}

impl Deref for PriceSourceCollector {
    type Target = dyn PriceSource;

    fn deref(&self) -> &Self::Target {
        match self {
            PriceSourceCollector::Pyth(pyth) => pyth,
        }
    }
}

pub trait PriceSource {
    fn get_price(&self, storage: &dyn Storage) -> anyhow::Result<PrecisionedPrice>;
}

// ------------------------------------ Pyth -----------------------------------

pub const PRICE_FEEDS: Map<PythId, PrecisionlessPrice> = Map::new("price_feeds");

#[grug::derive(Serde, Borsh)]
pub struct PythPriceSource {
    identifier: PythId,
    precision: u8,
}

impl PythPriceSource {
    pub fn new(id: PythId, precision: u8) -> Self {
        Self {
            identifier: id,
            precision,
        }
    }
}

impl PriceSource for PythPriceSource {
    fn get_price(&self, storage: &dyn Storage) -> anyhow::Result<PrecisionedPrice> {
        let price = PRICE_FEEDS.load(storage, self.identifier)?;
        Ok(price.with_precision(self.precision))
    }
}
