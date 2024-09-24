use crate::{Dec, Int};

/// Describes a type that wraps another type.
///
/// This trait is used in [`generate_uint!`](crate::generate_uint!) and
/// [`generate_decimal!`](crate::generate_decimal!) to get the inner type of a
/// [`Int`] and implement the conversion from the inner type to the [`Int`].
pub trait Inner {
    type U;
}

impl<U> Inner for Int<U> {
    type U = U;
}

impl<U> Inner for Dec<U> {
    type U = U;
}
