use {
    dango_types::oracle::{PrecisionlessPrice, PriceSource},
    grug::{Denom, Map, Serde, Timestamp},
    pyth_types::PythLazerId,
};

pub const PRICE_SOURCES: Map<&Denom, PriceSource, Serde> = Map::new("price_source");

pub const PYTH_TRUSTED_SIGNERS: Map<&[u8], Timestamp> = Map::new("pyth_trusted_signer");

pub const PYTH_PRICES: Map<PythLazerId, PrecisionlessPrice> = Map::new("pyth_price");
