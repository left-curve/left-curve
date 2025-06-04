use grug_math::{Dec, Int};

/// Describes a type that wraps another type.
///
/// This trait is used in [`generate_int!`](crate::generate_int!) and
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

impl<U, const S: u32> Inner for Dec<U, S> {
    type U = U;

    fn inner(&self) -> &Self::U {
        &self.0.0
    }

    fn into_inner(self) -> Self::U {
        self.0.0
    }
}

/// Describes a type that wraps another type, and the inner value is mutable.
pub trait InnerMut: Inner {
    /// Returns a mutable reference to the inner value.
    fn inner_mut(&mut self) -> &mut Self::U;
}
