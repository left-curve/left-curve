use crate::{
    Fraction, IsZero, MathError, MathResult, MultiplyRatio, Number, NumberConst, Sign, Uint,
};

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

impl<U, AsU, F> MultiplyFraction<F, AsU> for Uint<U>
where
    Uint<U>: NumberConst + Number + IsZero + MultiplyRatio + From<Uint<AsU>> + ToString,
    F: Number + Fraction<AsU> + Sign + ToString + IsZero,
{
    fn checked_mul_dec_floor(self, rhs: F) -> MathResult<Self> {
        // If either left or right hand side is zero, then simply return zero.
        if self.is_zero() || rhs.is_zero() {
            return Ok(Self::ZERO);
        }

        self.checked_multiply_ratio_floor(rhs.numerator(), F::denominator())
    }

    fn checked_mul_dec_ceil(self, rhs: F) -> MathResult<Self> {
        if self.is_zero() || rhs.is_zero() {
            return Ok(Self::ZERO);
        }

        self.checked_multiply_ratio_ceil(rhs.numerator(), F::denominator())
    }

    fn checked_div_dec_floor(self, rhs: F) -> MathResult<Self> {
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

        self.checked_multiply_ratio_floor(F::denominator(), rhs.numerator())
    }

    fn checked_div_dec_ceil(self, rhs: F) -> MathResult<Self> {
        if rhs.is_zero() {
            return Err(MathError::division_by_zero(self));
        }

        if self.is_zero() {
            return Ok(Self::ZERO);
        }

        self.checked_multiply_ratio_ceil(F::denominator(), rhs.numerator())
    }
}
