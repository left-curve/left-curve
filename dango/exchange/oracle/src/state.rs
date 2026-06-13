use {
    dango_primitives::{Denom, Timestamp},
    dango_pyth_types::PythId,
    dango_storage::{Map, Serde},
    dango_types::oracle::{Price, PriceConfig},
};

pub const PRICE_SOURCES: Map<&Denom, PriceConfig, Serde> = Map::new("price_source");

pub const PYTH_TRUSTED_SIGNERS: Map<&[u8], Timestamp> = Map::new("pyth_trusted_signer");

pub const PYTH_PRICES: Map<PythId, Price> = Map::new("pyth_price");
