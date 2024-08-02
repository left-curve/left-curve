use std::marker::PhantomData;

/// `Type State Builder` for `Intitialized` field but empty (never set/used/populated).
// #[derive(Default)]
pub struct TSBUnset<T>(PhantomData<T>);

impl<T> Default for TSBUnset<T> {
    fn default() -> Self {
        Self(Default::default())
    }
}

/// `Type State Builder` for `Intitialized` field but empty (never set/used/populated).
pub struct TSBEmpty<T>(pub T);

/// `Type State Builder` for `Intitialized` field.
pub struct TSBInit<T>(pub T);

/// `Type State Builder Reference` trait, to get access
/// to the `inner` value of [`TSBEmpty`] & [`TSBInit`].
///
/// Used when we want to access the `inner` value of the
/// regarding if the field is [`TSBEmpty`] or [`TSBInit`].
pub trait TSBRef {
    type I;
    fn inner(self) -> Self::I;
    fn borrow(&self) -> &Self::I;
}

impl<T> TSBRef for TSBEmpty<T> {
    type I = T;

    fn inner(self) -> Self::I {
        self.0
    }

    fn borrow(&self) -> &Self::I {
        &self.0
    }
}

impl<T> TSBRef for TSBInit<T> {
    type I = T;

    fn inner(self) -> Self::I {
        self.0
    }

    fn borrow(&self) -> &Self::I {
        &self.0
    }
}
