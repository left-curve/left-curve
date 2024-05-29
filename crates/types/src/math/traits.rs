use std::ops::{Add, Div};

use crate::{StdError, StdResult};

pub trait NumberConst {
    const MAX: Self;
    const MIN: Self;
    const ZERO: Self;
    const ONE: Self;
    const TEN: Self;
}

pub trait Bytable<const S: usize>: Sized {
    const LEN: usize = S;
    fn from_be_bytes(data: [u8; S]) -> Self;
    fn from_le_bytes(data: [u8; S]) -> Self;
    fn to_be_bytes(self) -> [u8; S];
    fn to_le_bytes(self) -> [u8; S];
    fn byte_len() -> usize {
        S
    }
    fn grow_be_bytes<const INPUT_SIZE: usize>(data: [u8; INPUT_SIZE]) -> [u8; S];
    fn grow_le_bytes<const INPUT_SIZE: usize>(data: [u8; INPUT_SIZE]) -> [u8; S];

    fn from_be_bytes_growing<const INPUT_SIZE: usize>(data: [u8; INPUT_SIZE]) -> Self {
        Self::from_be_bytes(Self::grow_be_bytes(data))
    }
    fn from_le_bytes_growing<const INPUT_SIZE: usize>(data: [u8; INPUT_SIZE]) -> Self {
        Self::from_le_bytes(Self::grow_le_bytes(data))
    }
}

pub trait CheckedOps: Sized {
    fn checked_add(self, other: Self) -> StdResult<Self>;
    fn checked_sub(self, other: Self) -> StdResult<Self>;
    fn checked_mul(self, other: Self) -> StdResult<Self>;
    fn checked_div(self, other: Self) -> StdResult<Self>;
    fn checked_rem(self, other: Self) -> StdResult<Self>;
    fn checked_pow(self, other: u32) -> StdResult<Self>;
    fn checked_shl(self, other: u32) -> StdResult<Self>;
    fn checked_shr(self, other: u32) -> StdResult<Self>;
    fn checked_ilog2(self) -> StdResult<u32>;
    fn checked_ilog10(self) -> StdResult<u32>;
    fn wrapping_add(self, other: Self) -> Self;
    fn wrapping_sub(self, other: Self) -> Self;
    fn wrapping_mul(self, other: Self) -> Self;
    fn wrapping_pow(self, other: u32) -> Self;
    fn saturating_add(self, other: Self) -> Self;
    fn saturating_sub(self, other: Self) -> Self;
    fn saturating_mul(self, other: Self) -> Self;
    fn saturating_pow(self, other: u32) -> Self;
    fn abs(self) -> Self;
}

pub trait NextNumber {
    type Next;
}

pub trait Sqrt: Sized {
    fn checked_sqrt(self) -> StdResult<Self>;
    fn sqrt(self) -> Self {
        self.checked_sqrt().unwrap()
    }
}

impl<T> Sqrt for T
where
    T: NumberConst
        + PartialEq
        + PartialOrd
        + Add<Output = Self>
        + Div<Output = Self>
        + Copy
        + ToString,
{
    fn checked_sqrt(self) -> StdResult<Self> {
        if self == Self::ZERO {
            return Ok(Self::ZERO);
        } else if self < Self::ZERO {
            return Err(StdError::negative_sqrt::<Self>(self));
        }

        let two = Self::ONE + Self::ONE;
        let mut x = self;
        let mut y = (x + Self::ONE) / two;
        while y < x {
            x = y;
            y = (x + self / x) / two;
        }
        Ok(x)
    }
}

#[cfg(test)]
mod test {
    use crate::{Int128, Sqrt, Uint128};

    #[test]
    fn sqrt() {
        let val = 100_u128;
        assert_eq!(val.sqrt(), 10_u128);
        let val = Uint128::new(64);
        assert_eq!(val.sqrt(), Uint128::new(8));

        let val = Int128::new(-64);
        val.checked_sqrt().unwrap_err();
    }
}
