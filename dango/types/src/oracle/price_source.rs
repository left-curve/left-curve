use pyth_types::{Channel, PythId};

#[grug::derive(Serde)]
pub struct PriceSource {
    /// The Pyth Lazer ID of the price feed.
    pub id: PythId,
    /// The channel of the Pyth Lazer price feed.
    pub channel: Channel,
}
