use {crate::oracle::PythId, grug::Udec128};

#[grug::derive(Serde, Borsh)]
pub enum PriceSource {
    /// A price source that uses a fixed price. For testing purposes only.
    Fixed {
        /// The price of the token.
        humanized_price: Udec128,
        /// The number of decimal places of the token that is used to convert
        /// the price from its smallest unit to a humanized form. E.g. 1 ATOM
        /// is 10^6 uatom, so the precision is 6.
        precision: u8,
        /// The timestamp of the price.
        timestamp: u64,
    },
    /// A price source that uses price feeds from Pyth.
    Pyth {
        /// The Pyth ID of the price.
        id: PythId,
        /// The number of decimal places of the token that is used to convert
        /// the price from its smallest unit to a humanized form. E.g. 1 ATOM
        /// is 10^6 uatom, so the precision is 6.
        precision: u8,
    },
    /// A price source for an LP token of the lending pool.
    LendingLiquidity,
}
