use crate::{Int, IsZero, MathResult, NextNumber, Number, NumberConst, PrevNumber};

/// Describes operations where a number is multiplied by a numerator then
/// immediately divided by a denominator.
/// This is different from applying a multiplication and a division sequentially,
/// because the multiplication part can overflow.
pub trait MultiplyRatio: Sized {
    fn checked_multiply_ratio_floor(self, numerator: Self, denominator: Self) -> MathResult<Self>;

    fn checked_multiply_ratio_ceil(self, numerator: Self, denominator: Self) -> MathResult<Self>;
}

impl<U> MultiplyRatio for Int<U>
where
    Int<U>: NextNumber + NumberConst + Number + Copy,
    <Int<U> as NextNumber>::Next: Number + IsZero + PrevNumber<Prev = Int<U>>,
{
    fn checked_multiply_ratio_floor(self, numerator: Self, denominator: Self) -> MathResult<Self> {
        self.checked_full_mul(numerator)?
            .checked_div(denominator.into_next())?
            .checked_into_prev()
    }

    fn checked_multiply_ratio_ceil(self, numerator: Self, denominator: Self) -> MathResult<Self> {
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
