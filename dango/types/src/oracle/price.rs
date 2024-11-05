use {
    grug::{Defined, Udec128, Undefined},
    pyth_sdk::PriceFeed,
};

pub type PrecisionlessPrice = Price<Undefined<u8>>;
pub type PrecisionedPrice = Price<Defined<u8>>;

#[grug::derive(Serde, Borsh)]
pub struct Price<P = Defined<u8>> {
    pub humanized_price: Udec128,
    pub humanized_ema: Udec128,
    pub timestamp: u64,
    precision: P,
}

impl PrecisionlessPrice {
    pub fn with_precision(self, precision: u8) -> Price<Defined<u8>> {
        Price {
            humanized_price: self.humanized_price,
            humanized_ema: self.humanized_ema,
            timestamp: self.timestamp,
            precision: Defined::new(precision),
        }
    }
}

impl PrecisionedPrice {
    pub fn precision(&self) -> u8 {
        self.precision.into_inner()
    }
}

impl TryFrom<PriceFeed> for PrecisionlessPrice {
    type Error = anyhow::Error;

    fn try_from(value: PriceFeed) -> Result<Self, Self::Error> {
        let price_unchecked = value.get_price_unchecked();
        let price = Udec128::checked_from_atomics(
            price_unchecked.price.unsigned_abs() as u128,
            price_unchecked.expo.unsigned_abs(),
        )?;

        let ema_unchecked = value.get_ema_price_unchecked();
        let ema = Udec128::checked_from_atomics(
            ema_unchecked.price.unsigned_abs() as u128,
            ema_unchecked.expo.unsigned_abs(),
        )?;

        Ok(Price {
            humanized_price: price,
            humanized_ema: ema,
            timestamp: price_unchecked.publish_time.unsigned_abs(),
            precision: Undefined::new(),
        })
    }
}
