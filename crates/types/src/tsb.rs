use std::marker::PhantomData;

/// `Type State Builder` for `unset` field.
pub struct TSBUnset<T>(PhantomData<T>);

// Need to implement Default manually
// because with derive macro it require to T to impelemt default (not needed).
impl<T> Default for TSBUnset<T> {
    fn default() -> Self {
        Self(Default::default())
    }
}

/// `Type State Builder` for `populated` field.
pub struct TSBInit<T>(pub T);

/// `Type State Builder Reference` trait, to get access
/// to the `inner` value of [`TSBUnset`] and [`TSBInit`].
///
/// Used when we want to access the `inner` value of the
/// regarding if the field is [`TSBUnset`] or [`TSBInit`].
pub trait TSBRef {
    type I;
    fn inner(self) -> Option<Self::I>;
}

impl<T> TSBRef for TSBInit<T> {
    type I = T;

    fn inner(self) -> Option<Self::I> {
        Some(self.0)
    }
}

impl<T> TSBRef for TSBUnset<T> {
    type I = T;

    fn inner(self) -> Option<Self::I> {
        None
    }
}
