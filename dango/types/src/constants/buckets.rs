use {
    crate::dex::Price,
    grug::{NonZero, Uint128},
};

const fn bucket(numerator: u128, denominator: u128, base: u32, quote: u32) -> NonZero<Price> {
    let exponent = 10_u128.pow(Price::DECIMAL_PLACES + quote - base);
    let raw = numerator * exponent / denominator;

    assert!(raw != 0, "bucket price can't be zero");

    NonZero::new_unchecked(Price::raw(Uint128::new(raw)))
}

pub mod dango_usdc {
    use {
        super::*,
        crate::constants::{dango, usdc},
    };

    pub const ONE_THOUSANDTH: NonZero<Price> = bucket(1, 1000, dango::DECIMAL, usdc::DECIMAL);

    pub const ONE_HUNDREDTH: NonZero<Price> = bucket(1, 100, dango::DECIMAL, usdc::DECIMAL);

    pub const ONE_TENTH: NonZero<Price> = bucket(1, 10, dango::DECIMAL, usdc::DECIMAL);

    pub const ONE: NonZero<Price> = bucket(1, 1, dango::DECIMAL, usdc::DECIMAL);

    pub const TEN: NonZero<Price> = bucket(10, 1, dango::DECIMAL, usdc::DECIMAL);

    pub const FIFTY: NonZero<Price> = bucket(50, 1, dango::DECIMAL, usdc::DECIMAL);

    pub const ONE_HUNDRED: NonZero<Price> = bucket(100, 1, dango::DECIMAL, usdc::DECIMAL);
}

pub mod btc_usdc {
    use {
        super::*,
        crate::constants::{btc, usdc},
    };

    pub const ONE_HUNDREDTH: NonZero<Price> = bucket(1, 100, btc::DECIMAL, usdc::DECIMAL);

    pub const ONE_TENTH: NonZero<Price> = bucket(1, 10, btc::DECIMAL, usdc::DECIMAL);

    pub const ONE: NonZero<Price> = bucket(1, 1, btc::DECIMAL, usdc::DECIMAL);

    pub const TEN: NonZero<Price> = bucket(10, 1, btc::DECIMAL, usdc::DECIMAL);

    pub const FIFTY: NonZero<Price> = bucket(50, 1, btc::DECIMAL, usdc::DECIMAL);

    pub const ONE_HUNDRED: NonZero<Price> = bucket(100, 1, btc::DECIMAL, usdc::DECIMAL);
}

pub mod eth_usdc {
    use {
        super::*,
        crate::constants::{eth, usdc},
    };

    pub const ONE_HUNDREDTH: NonZero<Price> = bucket(1, 100, eth::DECIMAL, usdc::DECIMAL);

    pub const ONE_TENTH: NonZero<Price> = bucket(1, 10, eth::DECIMAL, usdc::DECIMAL);

    pub const ONE: NonZero<Price> = bucket(1, 1, eth::DECIMAL, usdc::DECIMAL);

    pub const TEN: NonZero<Price> = bucket(10, 1, eth::DECIMAL, usdc::DECIMAL);

    pub const FIFTY: NonZero<Price> = bucket(50, 1, eth::DECIMAL, usdc::DECIMAL);

    pub const ONE_HUNDRED: NonZero<Price> = bucket(100, 1, eth::DECIMAL, usdc::DECIMAL);
}

pub mod sol_usdc {
    use {
        super::*,
        crate::constants::{sol, usdc},
    };

    pub const ONE_HUNDREDTH: NonZero<Price> = bucket(1, 100, sol::DECIMAL, usdc::DECIMAL);

    pub const ONE_TENTH: NonZero<Price> = bucket(1, 10, sol::DECIMAL, usdc::DECIMAL);

    pub const ONE: NonZero<Price> = bucket(1, 1, sol::DECIMAL, usdc::DECIMAL);

    pub const TEN: NonZero<Price> = bucket(10, 1, sol::DECIMAL, usdc::DECIMAL);
}

/// Some price buckets without considering base and quote decimal places.
/// For testing purpose.
pub mod mock {
    use super::*;

    /// 0.001
    pub const ONE_THOUSANDTH: Price = Price::raw(Uint128::new(1_000_000_000_000_000_000_000));

    /// 0.01
    pub const ONE_HUNDREDTH: Price = Price::raw(Uint128::new(10_000_000_000_000_000_000_000));

    /// 0.1
    pub const ONE_TENTH: Price = Price::raw(Uint128::new(100_000_000_000_000_000_000_000));

    pub const ONE: Price = Price::new(1);

    pub const TEN: Price = Price::new(10);

    pub const FIFTY: Price = Price::new(50);

    pub const ONE_HUNDRED: Price = Price::new(100);
}
