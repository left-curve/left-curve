use crate::MathResult;

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
