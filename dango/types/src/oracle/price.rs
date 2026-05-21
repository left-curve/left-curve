use {
    dango_order_book::UsdPrice,
    grug::{Dec128_6, Timestamp},
    pyth_types::{MarketSession, PayloadFeedData, PayloadPropertyValue},
};

#[grug::derive(Serde, Borsh)]
pub struct Price {
    /// The price of the token in its humanized form. I.e. the price of 1 ATOM,
    /// rather than 1 uatom.
    pub humanized_price: UsdPrice,

    /// The UNIX timestamp of the price (seconds since UNIX epoch).
    pub timestamp: Timestamp,

    /// The market session at which the price was observed. For 24/7 markets
    /// (e.g. crypto) this is always `Regular`. For markets with scheduled
    /// sessions (e.g. equities) this captures whether the feed was in regular
    /// trading hours or some other state (pre/post-market, overnight, closed).
    /// Falls back to `Other` when the feed payload omits the property.
    pub market_session: MarketSession,
}

impl Price {
    pub fn new(
        humanized_price: UsdPrice,
        timestamp: Timestamp,
        market_session: MarketSession,
    ) -> Self {
        Self {
            humanized_price,
            timestamp,
            market_session,
        }
    }
}

impl TryFrom<(PayloadFeedData, Timestamp)> for Price {
    type Error = anyhow::Error;

    fn try_from((feed_data, timestamp): (PayloadFeedData, Timestamp)) -> Result<Self, Self::Error> {
        let price = feed_data
            .properties
            .iter()
            .find_map(|property| {
                if let PayloadPropertyValue::Price(Some(price)) = property {
                    Some(price)
                } else {
                    None
                }
            })
            .ok_or_else(|| anyhow::anyhow!("price not found"))?;

        let exponent = feed_data
            .properties
            .iter()
            .find_map(|property| {
                if let PayloadPropertyValue::Exponent(exponent) = property {
                    Some(exponent)
                } else {
                    None
                }
            })
            .ok_or_else(|| anyhow::anyhow!("exponent not found"))?;

        // The `MarketSession` property is requested as part of the Pyth Lazer
        // subscription, so it should always be present in well-formed payloads.
        // We tolerate its absence by falling back to `Other` rather than
        // erroring, since a missing session classification is less critical
        // than a missing price or exponent.
        let market_session = feed_data
            .properties
            .iter()
            .find_map(|property| {
                if let PayloadPropertyValue::MarketSession(value) = property {
                    Some(MarketSession::from(*value))
                } else {
                    None
                }
            })
            .unwrap_or(MarketSession::Other);

        // Pyth Lazer prices come as `mantissa * 10^exponent`. The exponent is
        // negative for sub-integer precision (e.g. `-8` means the mantissa is
        // scaled by `10^8`). `Dec128_6::checked_from_atomics` interprets its
        // second argument as the number of decimal places to shift, so we pass
        // `-exponent` to recover the humanized value.
        let humanized_price = Dec128_6::checked_from_atomics::<i128>(
            price.mantissa_i64().into(),
            (-exponent).try_into()?,
        )?;

        Ok(Price {
            humanized_price: UsdPrice::new(humanized_price),
            timestamp,
            market_session,
        })
    }
}
