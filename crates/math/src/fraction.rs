use crate::{FixedPoint, MathResult, MultiplyRatio, Udec, Uint};

/// Describes a number that can be expressed as the quotient of two integers.
pub trait Fraction<U>: Sized {
    fn numerator(&self) -> Uint<U>;

    fn denominator() -> Uint<U>;

    fn checked_inv(&self) -> MathResult<Self>;
}

impl<U> Fraction<U> for Udec<U>
where
    Self: FixedPoint<U>,
    U: Copy,
    Uint<U>: MultiplyRatio,
{
    fn numerator(&self) -> Uint<U> {
        self.0
    }

    fn denominator() -> Uint<U> {
        Self::DECIMAL_FRACTION
    }

    fn checked_inv(&self) -> MathResult<Self> {
        Self::checked_from_ratio(Self::DECIMAL_FRACTION, self.0)
    }
}
