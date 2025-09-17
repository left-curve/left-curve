use {
    grug::{
        Dec, Defined, Exponentiate, FixedPoint, MathResult, MaybeDefined, MultiplyFraction, Number,
        NumberConst, PrevNumber, Timestamp, Udec128, Uint128, Uint256, Undefined,
    },
    pyth_types::{PayloadFeedData, PayloadPropertyValue, PriceFeed},
    std::cmp::Ordering,
};

pub type Precision = u8;

pub type PrecisionlessPrice = Price<Undefined<Precision>>;

pub type PrecisionedPrice = Price<Defined<Precision>>;

#[grug::derive(Serde, Borsh)]
pub struct Price<P>
where
    P: MaybeDefined<Precision>,
{
    /// The price of the token in its humanized form. I.e. the price of 1 ATOM,
    /// rather than 1 uatom.
    pub humanized_price: Udec128,
    /// The UNIX timestamp of the price (seconds since UNIX epoch).
    pub timestamp: Timestamp,
    /// The number of decimal places of the token that is used to convert
    /// the price from its smallest unit to a humanized form. E.g. 1 ATOM
    /// is 10^6 uatom, so the precision is 6.
    precision: P,
}

impl PrecisionlessPrice {
    /// Creates a new PrecisionlessPrice with the given humanized price.
    pub fn new(humanized_price: Udec128, timestamp: Timestamp) -> Self {
        Self {
            humanized_price,
            timestamp,
            precision: Undefined::new(),
        }
    }

    pub fn with_precision(self, precision: Precision) -> PrecisionedPrice {
        Price {
            humanized_price: self.humanized_price,
            timestamp: self.timestamp,
            precision: Defined::new(precision),
        }
    }
}

impl PrecisionedPrice {
    pub fn new(humanized_price: Udec128, timestamp: Timestamp, precision: Precision) -> Self {
        Self {
            humanized_price,
            timestamp,
            precision: Defined::new(precision),
        }
    }

    /// Returns the number of decimal places of the token that is used to
    /// convert the price from its smallest unit to a humanized form. E.g.
    /// 1 ATOM is 10^6 uatom, so the precision is 6.
    pub fn precision(&self) -> Precision {
        self.precision.into_inner()
    }

    /// Returns the value of a given unit amount. E.g. if this Price represents
    /// the price in USD of one ATOM, then this function will return the value
    /// in USD of the given number of uatom.
    ///
    /// e.g.
    /// Humanized price: 3000
    /// precision: 18
    /// unit amount: 1*10^18
    /// value: 3000 * 1*10^18 / 10^18 = 3000
    pub fn value_of_unit_amount<const S: u32>(
        &self,
        unit_amount: Uint128,
    ) -> MathResult<Dec<u128, S>>
    where
        Dec<u128, S>: FixedPoint<u128> + NumberConst,
    {
        self.humanized_price
            .convert_precision()?
            .checked_mul(Dec::<u128, S>::checked_from_ratio(
                unit_amount,
                10u128.pow(self.precision.into_inner() as u32),
            )?)
    }

    pub fn value_of_dec_amount<const S1: u32, const S2: u32>(
        &self,
        dec_amount: Dec<u128, S1>,
    ) -> MathResult<Dec<u128, S2>> {
        let mut num = dec_amount.0.checked_full_mul(self.humanized_price.0)?;

        match S1.cmp(&S2) {
            Ordering::Less => {
                let diff = Uint256::TEN.checked_pow(S2 - S1)?;
                num.checked_mul_assign(diff)?;
            },
            Ordering::Greater => {
                let diff = Uint256::TEN.checked_pow(S1 - S2)?;
                num.checked_div_assign(diff)?;
            },
            Ordering::Equal => {},
        }

        Dec::raw(
            num.checked_div(Dec::<_, 18>::PRECISION)?
                .checked_div(Uint256::TEN.checked_pow(self.precision.into_inner() as u32)?)?,
        )
        .checked_into_prev()
    }

    /// Returns the unit amount of a given value. E.g. if this Price represents
    /// the price in USD of one ATOM, then this function will return the amount
    /// in uatom of the given USD value.
    ///
    /// e.g.
    /// Humanized price: 3000
    /// precision: 18
    /// value: 1000
    /// unit amount: 1000 / 3000 * 10^18 = 1000*10^18 / 3000 = 3.33*10^17
    pub fn unit_amount_from_value(&self, value: Udec128) -> MathResult<Uint128> {
        Uint128::new(10u128.pow(self.precision.into_inner() as u32))
            .checked_mul_dec(value.checked_div(self.humanized_price)?)
    }

    /// Returns the unit amount of a given value, rounded up.
    pub fn unit_amount_from_value_ceil(&self, value: Udec128) -> MathResult<Uint128> {
        Uint128::new(10u128.pow(self.precision.into_inner() as u32))
            .checked_mul_dec_ceil(value.checked_div(self.humanized_price)?)
    }
}

impl TryFrom<PriceFeed> for PrecisionlessPrice {
    type Error = anyhow::Error;

    fn try_from(value: PriceFeed) -> Result<Self, Self::Error> {
        let price_unchecked = value.get_price_unchecked();
        let price = Udec128::checked_from_atomics::<u128>(
            price_unchecked.price.try_into()?,
            (-price_unchecked.expo).try_into()?,
        )?;

        let timestamp = Timestamp::from_seconds(price_unchecked.publish_time.try_into()?);

        Ok(Price {
            humanized_price: price,
            timestamp,
            precision: Undefined::new(),
        })
    }
}

impl TryFrom<(PayloadFeedData, Timestamp)> for PrecisionlessPrice {
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

        let price = Udec128::checked_from_atomics::<u128>(
            price.into_inner().get().try_into()?,
            (-exponent).try_into()?,
        )?;

        Ok(Price {
            humanized_price: price,
            timestamp,
            precision: Undefined::new(),
        })
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        grug::{IsZero, Udec128_24},
    };

    #[test]
    fn value_of_unit_amount_does_not_overflow_with_large_precision() {
        // $100M per ETH
        let price = PrecisionedPrice {
            humanized_price: Udec128::new(100_000_000u128),
            timestamp: Timestamp::from_seconds(0),
            precision: Defined::new(18),
        };

        // Value of 100M ETH at $100M = $100000T
        let value = price
            .value_of_unit_amount(Uint128::new(100_000_000u128 * 10u128.pow(18)))
            .unwrap();
        assert_eq!(value, Udec128::new(10_000_000_000_000_000u128));
    }

    #[test]
    fn unit_amount_from_value_does_not_overflow_with_large_precision() {
        // $100M per ETH
        let price = PrecisionedPrice {
            humanized_price: Udec128::new(100_000_000u128),
            timestamp: Timestamp::from_seconds(0),
            precision: Defined::new(18),
        };

        // Value of 100M ETH at $100M = $100000T
        let unit_amount = price
            .unit_amount_from_value(Udec128::new(10_000_000_000_000_000u128))
            .unwrap();
        assert_eq!(unit_amount, Uint128::new(100_000_000u128 * 10u128.pow(18)));
    }

    #[test]
    fn value_of_unit_amount_works_with_large_precision_and_small_price() {
        // 0.000001 USD per token
        let price = PrecisionedPrice {
            humanized_price: Udec128::checked_from_ratio(1, 1_000_000).unwrap(),
            timestamp: Timestamp::from_seconds(0),
            precision: Defined::new(18),
        };

        // Value of 1 unit of token at 0.000001 USD = 0.000001 / 10^18 USD
        let value: Udec128_24 = price.value_of_unit_amount(Uint128::new(1)).unwrap();
        println!("value: {value}");
        assert!(value.is_non_zero());
        assert_eq!(
            value,
            Udec128_24::checked_from_ratio(1, 1_000_000 * 10u128.pow(18)).unwrap()
        );
    }
}
