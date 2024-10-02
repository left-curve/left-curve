use crate::{Dec, FixedPoint, MathResult, Number};

/// Describes operations that decimal types must implement, which may not be
/// relevant for non-decimal types.
pub trait Decimal: Sized {
    fn checked_floor(self) -> MathResult<Self>;

    fn checked_ceil(self) -> MathResult<Self>;
}

impl<U> Decimal for Dec<U>
where
    Self: FixedPoint<U>,
    U: Number + Copy + PartialEq,
{
    fn checked_floor(self) -> MathResult<Self> {
        // There are two ways to floor:
        // 1. inner / decimal_fraction * decimal_fraction
        // 2. inner - inner % decimal_fraction
        // Method 2 is faster because Rem is roughly as fast as or slightly
        // faster than Div, while Sub is significantly faster than Mul.
        //
        // This flooring operation in fact can never fail, because flooring an
        // unsigned decimal goes down to 0 at most. However, flooring a _signed_
        // decimal may underflow.
        Ok(Self(self.0 - self.0.checked_rem(Self::DECIMAL_FRACTION)?))
    }

    fn checked_ceil(self) -> MathResult<Self> {
        let floor = self.checked_floor()?;
        if floor == self {
            Ok(floor)
        } else {
            floor.0.checked_add(Self::DECIMAL_FRACTION).map(Self)
        }
    }
}
