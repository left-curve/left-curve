use crate::{Fraction, MathResult};

/// Describes operations between a number and a decimal type.
pub trait MultiplyFraction<F, U>: Sized
where
    F: Fraction<U>,
{
    fn checked_mul_dec_floor(self, rhs: F) -> MathResult<Self>;

    fn checked_mul_dec_ceil(self, rhs: F) -> MathResult<Self>;

    fn checked_div_dec_floor(self, rhs: F) -> MathResult<Self>;

    fn checked_div_dec_ceil(self, rhs: F) -> MathResult<Self>;
}
