use crate::{
    Bytable, Dec128, Dec256, Inner, Int128, Int256, Int512, Int64, Udec128, Udec256, Uint128,
    Uint256, Uint512, Uint64,
};

/// Describes a number type can be cast into another type of a bigger word size.
///
/// For example, [`Uint128`](crate::Uint128) can be safety cast to [`Uint256`](crate::Uint256).
/// In this case, [`NextNumber`] trait should be implemented for [`Uint128`](crate::Uint128)
/// with `Next` being [`Uint256`](crate::Uint256).
pub trait NextNumber {
    type Next;

    fn into_next(self) -> Self::Next;
}

// ------------------------------------ std ------------------------------------

macro_rules! impl_next {
    ($this:ty => $next:ty) => {
        impl NextNumber for $this {
            type Next = $next;

            fn into_next(self) -> Self::Next {
                <$next>::new(self.0.into())
            }
        }
    };
    ($($this:ty => $next:ty),+ $(,)?) => {
        $(
            impl_next!($this => $next);
        )+
    };
}

impl_next! {
    Uint64  => Uint128,
    Uint128 => Uint256,
    Int64   => Int128,
    Int128  => Int256,
}

// ----------------------------------- bnum ------------------------------------

macro_rules! impl_next_bnum {
    ($this:ty => $next:ty) => {
        impl NextNumber for $this {
            type Next = $next;

            fn into_next(self) -> Self::Next {
                <$next>::new(<$next as Inner>::U::from_be_bytes_growing(
                    self.0.to_be_bytes(),
                ))
            }
        }
    };
    ($($this:ty => $next:ty),+ $(,)?) => {
        $(
            impl_next_bnum!($this => $next);
        )+
    };
}

impl_next_bnum! {
    Uint256 => Uint512,
    Int256  => Int512,
}

// ----------------------------------- dec ------------------------------------

macro_rules! impl_next_udec {
    ($this:ty => $next:ty) => {
        impl NextNumber for $this {
            type Next = $next;

            fn into_next(self) -> Self::Next {
                <$next>::raw(self.0.into_next())
            }
        }
    };
    ($($this:ty => $next:ty),+ $(,)?) => {
        $(
            impl_next_udec!($this => $next);
        )+
    };
}

impl_next_udec! {
    Udec128 => Udec256,
    Dec128  => Dec256,
}
