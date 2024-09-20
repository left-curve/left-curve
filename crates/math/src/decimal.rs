use crate::MathResult;

/// Describes operations that decimal types must implement, which may not be
/// relevant for non-decimal types.
pub trait Decimal: Sized {
    fn checked_floor(self) -> MathResult<Self>;

    fn checked_ceil(self) -> MathResult<Self>;
}
