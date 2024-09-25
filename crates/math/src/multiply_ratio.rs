use crate::{IsZero, MathError, MathResult, NextNumber, Number, NumberConst, Uint};

/// Describes operations where a number is multiplied by a numerator then
/// immediately divided by a denominator.
/// This is different from applying a multiplication and a division sequentially,
/// because the multiplication part can overflow.
pub trait MultiplyRatio: Sized {
    fn checked_multiply_ratio_floor<A, B>(self, numerator: A, denominator: B) -> MathResult<Self>
    where
        A: Into<Self>,
        B: Into<Self>;

    fn checked_multiply_ratio_ceil<A, B>(self, numerator: A, denominator: B) -> MathResult<Self>
    where
        A: Into<Self>,
        B: Into<Self>;
}

impl<U> MultiplyRatio for Uint<U>
where
    Uint<U>: NextNumber + NumberConst + Number + Copy,
    <Uint<U> as NextNumber>::Next: Number + IsZero + ToString + Clone,
{
    fn checked_multiply_ratio_floor<A: Into<Self>, B: Into<Self>>(
        self,
        numerator: A,
        denominator: B,
    ) -> MathResult<Self> {
        let denominator = denominator.into().into_next();
        let next_result = self.checked_full_mul(numerator)?.checked_div(denominator)?;
        next_result
            .clone()
            .try_into()
            .map_err(|_| MathError::overflow_conversion::<_, Self>(next_result))
    }

    fn checked_multiply_ratio_ceil<A: Into<Self>, B: Into<Self>>(
        self,
        numerator: A,
        denominator: B,
    ) -> MathResult<Self> {
        let numerator: Self = numerator.into();
        let denominator: Self = denominator.into();
        let dividend = self.checked_full_mul(numerator)?;
        let floor_result = self.checked_multiply_ratio_floor(numerator, denominator)?;
        let remained = dividend.checked_rem(denominator.into_next())?;
        if !remained.is_zero() {
            floor_result.checked_add(Self::ONE)
        } else {
            Ok(floor_result)
        }
    }
}
