use {
    crate::oracle::Precision,
    grug::{Timestamp, Udec128},
    pyth_types::{PythId, PythLazerId},
};

#[grug::derive(Serde, Borsh)]
pub enum PriceSource {
    /// A price source that uses a fixed price. For testing purposes only.
    Fixed {
        /// The price of the token.
        humanized_price: Udec128,
        /// The number of decimal places of the token that is used to convert
        /// the price from its smallest unit to a humanized form. E.g. 1 ATOM
        /// is 10^6 uatom, so the precision is 6.
        precision: Precision,
        /// The timestamp of the price.
        timestamp: Timestamp,
    },
    /// A price source that uses price feeds from Pyth.
    Pyth {
        /// The Pyth ID of the price.
        id: PythId,
        /// The number of decimal places of the token that is used to convert
        /// the price from its smallest unit to a humanized form. E.g. 1 ATOM
        /// is 10^6 uatom, so the precision is 6.
        precision: Precision,
    },
    PythLazer {
        /// The Pyth Lazer ID of the price feed.
        id: PythLazerId,
        /// The number of decimal places of the token that is used to convert
        /// the price from its smallest unit to a humanized form. E.g. 1 ATOM
        /// is 10^6 uatom, so the precision is 6.
        precision: Precision,
    },
    /// A price source for an LP token of the lending pool.
    LendingLiquidity,
}
