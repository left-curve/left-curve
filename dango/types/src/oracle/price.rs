use {
    grug::{Defined, Inner, Udec128, Uint128, Undefined},
    pyth_sdk::PriceFeed,
};

pub type PrecisionlessPrice = Price<Undefined<u8>>;
pub type PrecisionedPrice = Price<Defined<u8>>;

#[grug::derive(Serde, Borsh)]
pub struct Price<P = Defined<u8>> {
    /// The price of the token in its humanized form. I.e. the price of 1 ATOM, rather than 1 uatom.
    pub humanized_price: Udec128,
    /// The exponential moving average of the price of the token in its humanized form.
    pub humanized_ema: Udec128,
    /// The UNIX timestamp of the price (seconds since UNIX epoch).
    pub timestamp: u64,
    /// The number of decimal places of the token that is used to convert
    /// the price from its smallest unit to a humanized form. E.g. 1 ATOM
    /// is 10^6 uatom, so the precision is 6.
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
    /// Returns the number of decimal places of the token that is used to convert
    /// the price from its smallest unit to a humanized form. E.g. 1 ATOM
    /// is 10^6 uatom, so the precision is 6.
    pub fn precision(&self) -> u8 {
        self.precision.into_inner()
    }

    /// Returns the value of a given unit amount. E.g. if this Price represents
    /// the price in USD of one ATOM, then this function will return the value in USD of
    /// the given number of uatom.
    pub fn value_of_unit_amount(&self, unit_amount: Uint128) -> Udec128 {
        self.humanized_price * Udec128::new(*unit_amount.inner())
            / Udec128::new(10u128.pow(self.precision.into_inner() as u32))
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
