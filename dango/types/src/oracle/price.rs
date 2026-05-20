use {
    dango_order_book::UsdPrice,
    grug::{Dec128_6, Timestamp},
    pyth_types::{PayloadFeedData, PayloadPropertyValue},
};

#[grug::derive(Serde, Borsh)]
pub struct Price {
    /// The price of the token in its humanized form. I.e. the price of 1 ATOM,
    /// rather than 1 uatom.
    pub humanized_price: UsdPrice,
    /// The UNIX timestamp of the price (seconds since UNIX epoch).
    pub timestamp: Timestamp,
}

impl Price {
    pub fn new(humanized_price: UsdPrice, timestamp: Timestamp) -> Self {
        Self {
            humanized_price,
            timestamp,
        }
    }
}

impl TryFrom<(PayloadFeedData, Timestamp)> for Price {
    type Error = anyhow::Error;

    fn try_from((feed_data, timestamp): (PayloadFeedData, Timestamp)) -> Result<Self, Self::Error> {
        let price = feed_data.properties.iter().find_map(|property| {
            if let PayloadPropertyValue::Price(Some(price)) = property {
                Some(price)
            } else {
                None
            }
        });

        let exponent = feed_data.properties.iter().find_map(|property| {
            if let PayloadPropertyValue::Exponent(exponent) = property {
                Some(exponent)
            } else {
                None
            }
        });

        let price = price.ok_or_else(|| anyhow::anyhow!("price not found"))?;
        let exponent = exponent.ok_or_else(|| anyhow::anyhow!("exponent not found"))?;

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
        })
    }
}
