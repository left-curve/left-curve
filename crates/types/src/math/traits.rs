use crate::StdResult;

pub trait GrugNumber {
    const MAX: Self;
    const MIN: Self;
    const ZERO: Self;
    const ONE: Self;
}

pub trait Bytable<const S: usize> {
    fn from_be_bytes(data: [u8; S]) -> Self;
    fn from_le_bytes(data: [u8; S]) -> Self;
    fn to_be_bytes(self) -> [u8; S];
    fn to_le_bytes(self) -> [u8; S];
    fn byte_len() -> usize {
        S
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
}

pub trait NextNumer {
    type Next;
}
