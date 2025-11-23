use grug::{Int128, IsZero, Sign};

/// Returns true if `a` and `b` are both positive or both negative, or if both are zero.
pub fn same_side(a: Int128, b: Int128) -> bool {
    (a.is_positive() == b.is_positive()) || a.is_zero() || b.is_zero()
}
