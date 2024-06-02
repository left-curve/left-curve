use {
    crate::{
        grow_be_int, grow_be_uint, grow_le_int, grow_le_uint, impl_bytable_bnum,
        impl_bytable_ibnum, impl_bytable_std, impl_checked_ops, impl_checked_ops_signed,
        impl_checked_ops_unsigned, impl_number_const, Int, StdError, StdResult,
    },
    bnum::types::{I256, I512, U256, U512},
    std::ops::{Add, Div},
};

/// Describes the inner type of the [`Int`].
///
/// This trait is used in [`generate_int!`](crate::generate_int!) and
/// [`generate_decimal!`](crate::generate_decimal!) to get the inner type of a
/// [`Int`] and implement the conversion from the inner type to the [`Int`].
pub trait Inner {
    type U;
}

/// Describes a number type can be casted to another type of a bigger word size.
///
/// For example, [`Uint128`] can be safety cast to [`Uint256`]. In this case,
/// [`NextNumber`] trait should be implemented for [`Uint128`] with `Next` being
/// [`Uint256`].
pub trait NextNumber {
    type Next;
}

/// Describes a fixed-point number, which is represented by a numerator divided
/// by a constant denominator.
pub trait DecimalRef<U: NumberConst + CheckedOps> {
    fn numerator(self) -> Int<U>;

    fn denominator() -> Int<U>;
}

// ------------------------------- number const --------------------------------

pub trait NumberConst {
    const MAX: Self;
    const MIN: Self;
    const ZERO: Self;
    const ONE: Self;
    const TEN: Self;
}

impl_number_const!(u64, 0, u64::MAX, 0, 1, 10);
impl_number_const!(u128, 0, u128::MAX, 0, 1, 10);
impl_number_const!(U256, U256::MIN, U256::MAX, U256::ZERO, U256::ONE, U256::TEN);
impl_number_const!(U512, U512::MIN, U512::MAX, U512::ZERO, U512::ONE, U512::TEN);

impl_number_const!(i64, 0, i64::MAX, 0, 1, 10);
impl_number_const!(i128, 0, i128::MAX, 0, 1, 10);
impl_number_const!(I256, I256::MIN, I256::MAX, I256::ZERO, I256::ONE, I256::TEN);
impl_number_const!(I512, I512::MIN, I512::MAX, I512::ZERO, I512::ONE, I512::TEN);

// ---------------------------------- bytable ----------------------------------

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

impl_bytable_std!(u64, 8);
impl_bytable_std!(u128, 16);
impl_bytable_bnum!(U256, 32);
impl_bytable_bnum!(U512, 64);

impl_bytable_std!(i64, 8);
impl_bytable_std!(i128, 16);
impl_bytable_ibnum!(I256, 32, U256);
impl_bytable_ibnum!(I512, 64, U512);

// -------------------------------- checked ops --------------------------------

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

    fn is_zero(self) -> bool;
}

impl_checked_ops_unsigned!(u64);
impl_checked_ops_unsigned!(u128);
impl_checked_ops_unsigned!(U256);
impl_checked_ops_unsigned!(U512);

impl_checked_ops_signed!(i64);
impl_checked_ops_signed!(i128);
impl_checked_ops_signed!(I256);
impl_checked_ops_signed!(I512);

// -------------------------------- square root --------------------------------

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
    /// Computes an integer's square root using [Heron's method](https://en.wikipedia.org/wiki/Methods_of_computing_square_roots#Heron's_method).
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

// --------------------------- flooring and ceiling ----------------------------

pub trait IntPerDec<U, AsU, DR>: Sized
where
    Int<AsU>: Into<Int<U>>,
    DR: DecimalRef<AsU>,
    AsU: NumberConst + CheckedOps,
{
    fn checked_mul_dec_floor(self, rhs: DR) -> StdResult<Self>;

    fn checked_mul_dec_ceil(self, rhs: DR) -> StdResult<Self>;

    fn checked_div_dec_floor(self, rhs: DR) -> StdResult<Self>;

    fn checked_div_dec_ceil(self, rhs: DR) -> StdResult<Self>;

    fn mul_dec_floor(self, rhs: DR) -> Self {
        self.checked_mul_dec_floor(rhs).unwrap()
    }

    fn mul_dec_ceil(self, rhs: DR) -> Self {
        self.checked_mul_dec_ceil(rhs).unwrap()
    }

    fn div_dec_floor(self, rhs: DR) -> Self {
        self.checked_div_dec_floor(rhs).unwrap()
    }

    fn div_dec_ceil(self, rhs: DR) -> Self {
        self.checked_div_dec_ceil(rhs).unwrap()
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use crate::{Int128, Sqrt, Uint128};

    #[test]
    fn sqrt() {
        let val: u128 = 100;
        assert_eq!(val.sqrt(), 10);

        let val = Uint128::new(64);
        assert_eq!(val.sqrt(), Uint128::new(8));

        let val = Int128::new(-64);
        val.checked_sqrt().unwrap_err();
    }
}
