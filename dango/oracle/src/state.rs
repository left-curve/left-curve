use {
    dango_types::oracle::{Price, PriceSourceWithWeight},
    grug_storage::{Map, Serde},
    grug_types::{Denom, Timestamp},
    pyth_types::PythId,
};

pub const PRICE_SOURCES: Map<&Denom, Vec<PriceSourceWithWeight>, Serde> = Map::new("price_source");

pub const PYTH_TRUSTED_SIGNERS: Map<&[u8], Timestamp> = Map::new("pyth_trusted_signer");

pub const PYTH_PRICES: Map<PythId, Price> = Map::new("pyth_price");
