use crate::{Dec, Fraction, Int, IsZero, MathError, MathResult, MultiplyRatio, NumberConst};

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

impl<U> MultiplyFraction<Dec<U>, U> for Int<U>
where
    Int<U>: IsZero + NumberConst + MultiplyRatio + ToString + Copy,
    Dec<U>: IsZero + Fraction<U>,
{
    fn checked_mul_dec_floor(self, rhs: Dec<U>) -> MathResult<Self> {
        // If either left or right hand side is zero, then simply return zero.
        if self.is_zero() || rhs.is_zero() {
            return Ok(Self::ZERO);
        }

        self.checked_multiply_ratio_floor(*rhs.numerator(), Dec::<U>::denominator())
    }

    fn checked_mul_dec_ceil(self, rhs: Dec<U>) -> MathResult<Self> {
        if self.is_zero() || rhs.is_zero() {
            return Ok(Self::ZERO);
        }

        self.checked_multiply_ratio_ceil(*rhs.numerator(), Dec::<U>::denominator())
    }

    fn checked_div_dec_floor(self, rhs: Dec<U>) -> MathResult<Self> {
        // If right hand side is zero, throw error, because you can't divide any
        // number by zero.
        if rhs.is_zero() {
            return Err(MathError::division_by_zero(self));
        }

        // If left hand side is zero, and we know right hand size is positive,
        // then simply return zero.
        if self.is_zero() {
            return Ok(Self::ZERO);
        }

        self.checked_multiply_ratio_floor(Dec::<U>::denominator(), *rhs.numerator())
    }

    fn checked_div_dec_ceil(self, rhs: Dec<U>) -> MathResult<Self> {
        if rhs.is_zero() {
            return Err(MathError::division_by_zero(self));
        }

        if self.is_zero() {
            return Ok(Self::ZERO);
        }

        self.checked_multiply_ratio_ceil(Dec::<U>::denominator(), *rhs.numerator())
    }
}
