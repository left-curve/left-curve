use crate::{Dec, Int};

/// Describes a type that wraps another type.
///
/// This trait is used in [`generate_uint!`](crate::generate_uint!) and
/// [`generate_decimal!`](crate::generate_decimal!) to get the inner type of a
/// [`Int`] and implement the conversion from the inner type to the [`Int`].
pub trait Inner {
    type U;

    /// Returns an immutable reference to the inner value.
    fn inner(&self) -> &Self::U;

    /// Consume the wrapper, return an owned instance of the inner value.
    fn into_inner(self) -> Self::U;
}

impl<U> Inner for Int<U> {
    type U = U;

    fn inner(&self) -> &Self::U {
        &self.0
    }

    fn into_inner(self) -> Self::U {
        self.0
    }
}

impl<U> Inner for Dec<U> {
    type U = U;

    fn inner(&self) -> &Self::U {
        self.0.inner()
    }

    fn into_inner(self) -> Self::U {
        self.0.into_inner()
    }
}

/// Describes a type that wraps another type, and the inner value is mutable.
pub trait InnerMut: Inner {
    /// Returns a mutable reference to the inner value.
    fn inner_mut(&mut self) -> &mut Self::U;
}
