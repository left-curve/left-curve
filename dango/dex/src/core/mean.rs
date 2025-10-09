use grug::{
    Exponentiate, MathResult, NextNumber, Number, PrevNumber, Udec128_24, Udec256_24, Uint128,
};

pub fn arithmetic_mean(a: Udec128_24, b: Udec128_24) -> MathResult<Udec128_24> {
    // First try computing the mean in 128-bit. If it doesn't work,
    // try escalating to 256-bit.
    arithmetic_mean_128(a, b).or_else(|_| arithmetic_mean_256(a, b))
}

fn arithmetic_mean_128(a: Udec128_24, b: Udec128_24) -> MathResult<Udec128_24> {
    const HALF: Udec128_24 = Udec128_24::new_percent(50);

    a.checked_add(b)?.checked_mul(HALF)
}

/// For use when `a + b > Udec128_24::MAX`. Escalate both variables to 256-bit,
/// calculates the mean, and go down to 128-bit.
fn arithmetic_mean_256(a: Udec128_24, b: Udec128_24) -> MathResult<Udec128_24> {
    const HALF: Udec256_24 = Udec256_24::new_percent(50);

    a.into_next()
        .checked_add(b.into_next())?
        .checked_mul(HALF)?
        .checked_into_prev()
}

pub fn geometric_mean(a: Uint128, b: Uint128) -> MathResult<Uint128> {
    geometric_mean_128(a, b).or_else(|_| geometric_mean_256(a, b))
}

fn geometric_mean_128(a: Uint128, b: Uint128) -> MathResult<Uint128> {
    a.checked_mul(b)?.checked_sqrt()
}

fn geometric_mean_256(a: Uint128, b: Uint128) -> MathResult<Uint128> {
    a.into_next()
        .checked_mul(b.into_next())?
        .checked_sqrt()?
        .checked_into_prev()
}
