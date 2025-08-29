use grug::{Udec128_24, Uint128};

/// 0.001
pub const ONE_THOUSANDTH: Udec128_24 = Udec128_24::raw(Uint128::new(1_000_000_000_000_000_000_000));

/// 0.01
pub const ONE_HUNDREDTH: Udec128_24 = Udec128_24::raw(Uint128::new(10_000_000_000_000_000_000_000));

/// 0.1
pub const ONE_TENTH: Udec128_24 = Udec128_24::raw(Uint128::new(100_000_000_000_000_000_000_000));

pub const ONE: Udec128_24 = Udec128_24::new(1);

pub const TEN: Udec128_24 = Udec128_24::new(10);

pub const FIFTY: Udec128_24 = Udec128_24::new(50);

pub const ONE_HUNDRED: Udec128_24 = Udec128_24::new(100);
