use {
    crate::oracle::Precision,
    pyth_types::{Channel, PythId},
};

#[grug::derive(Serde)]
pub struct PriceSource {
    /// The Pyth Lazer ID of the price feed.
    pub id: PythId,
    /// The channel of the Pyth Lazer price feed.
    pub channel: Channel,
    /// The number of decimal places of the token that is used to convert
    /// the price from its smallest unit to a humanized form. E.g. 1 ATOM
    /// is 10^6 uatom, so the precision is 6.
    pub precision: Precision,
}
