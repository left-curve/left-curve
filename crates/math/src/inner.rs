use crate::{Udec, Uint};

/// Describes a type that wraps another type.
///
/// This trait is used in [`generate_uint!`](crate::generate_uint!) and
/// [`generate_decimal!`](crate::generate_decimal!) to get the inner type of a
/// [`Uint`] and implement the conversion from the inner type to the [`Uint`].
pub trait Inner {
    type U;
}

impl<U> Inner for Uint<U> {
    type U = U;
}
impl<U> Inner for Udec<U> {
    type U = U;
}
