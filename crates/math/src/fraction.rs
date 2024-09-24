use crate::{Dec, FixedPoint, Int, MathResult, MultiplyRatio};

/// Describes a number that can be expressed as the quotient of two integers.
///
/// Note that here we only concern the fraction's absolute value. Both the
/// numerator and denominator here are negative. This trait is intended to be
/// used together with [`Sign`] To account for negative fractions.
pub trait Fraction<U>: Sized {
    fn numerator(&self) -> Int<U>;

    fn denominator() -> Int<U>;

    fn checked_inv(&self) -> MathResult<Self>;
}

impl<U> Fraction<U> for Dec<U>
where
    Self: FixedPoint<U>,
    U: Copy,
    Int<U>: MultiplyRatio,
{
    fn numerator(&self) -> Int<U> {
        self.0
    }

    fn denominator() -> Int<U> {
        Self::DECIMAL_FRACTION
    }

    fn checked_inv(&self) -> MathResult<Self> {
        Self::checked_from_ratio(Self::DECIMAL_FRACTION, self.0)
    }
}
