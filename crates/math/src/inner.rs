use crate::{Udec, Uint};

/// Describes a type that wraps another type.
pub trait Inner {
    type U;

    // Returns an immutable reference to the inner value.
    fn inner(&self) -> &Self::U;

    // Consume the wrapper, return an owned instance of the inner value.
    fn into_inner(self) -> Self::U;
}

impl<U> Inner for Uint<U> {
    type U = U;

    fn inner(&self) -> &Self::U {
        &self.0
    }

    fn into_inner(self) -> Self::U {
        self.0
    }
}

impl<U> Inner for Udec<U> {
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
    // Returns a mutable reference to the inner value.
    fn inner_mut(&mut self) -> &mut Self::U;
}
