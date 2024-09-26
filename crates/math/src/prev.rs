use {
    crate::{
        Dec128, Dec256, Inner, Int128, Int256, Int512, Int64, MathError, MathResult, Udec128,
        Udec256, Uint128, Uint256, Uint512, Uint64,
    },
    bnum::BTryFrom,
};

/// Describes a number type can be cast into another type of a smaller word size.
///
/// For example, [`Uint256`](crate::Uint256) can be cast to [`Uint128`](crate::Uint128).
/// In this case, [`PrevNumber`] trait should be implemented for [`Uint256`](crate::Uint256)
/// with `Prev` being [`Uint128`](crate::Uint128).
pub trait PrevNumber {
    type Prev;
    fn into_prev(self) -> MathResult<Self::Prev>;
}

// ------------------------------------ std ------------------------------------

macro_rules! impl_prev {
    ($this:ty => $prev:ty) => {
        impl PrevNumber for $this {
            type Prev = $prev;

            fn into_prev(self) -> MathResult<Self::Prev> {
                self.0.try_into().map(<$prev>::new).map_err(|_| {
                    MathError::overflow_conversion::<_, $prev>(self)
                })
            }
        }
    };
    ($($this:ty => $prev:ty),+ $(,)?) => {
        $(
            impl_prev!($this => $prev);
        )+
    };
}

impl_prev! {
    Uint128 => Uint64,
    Uint256 => Uint128,
    Int128  => Int64,
    Int256  => Int128,
}

// ----------------------------------- bnum ------------------------------------

macro_rules! impl_prev_bnum {
    ($this:ty => $prev:ty) => {
        impl PrevNumber for $this {
            type Prev = $prev;

            fn into_prev(self) -> MathResult<Self::Prev> {
                BTryFrom::<<$this as Inner>::U>::try_from(self.0)
                    .map(<$prev>::new)
                    .map_err(|_| MathError::overflow_conversion::<_, Uint256>(self))
            }
        }
    };
    ($($this:ty => $prev:ty),+ $(,)?) => {
        $(
            impl_prev_bnum!($this => $prev);
        )+
    };
}

impl_prev_bnum! {
    Uint512 => Uint256,
    Int512  => Int256,
}

// ----------------------------------- dec ------------------------------------

macro_rules! impl_prev_dec {
    ($this:ty => $prev:ty) => {
        impl PrevNumber for $this {
            type Prev = $prev;

            fn into_prev(self) -> MathResult<Self::Prev> {
                self.0.into_prev().map(<$prev>::raw)
            }
        }
    };
    ($($this:ty => $prev:ty),+ $(,)?) => {
        $(
            impl_prev_dec!($this => $prev);
        )+
    };
}

impl_prev_dec! {
    Udec256 => Udec128,
    Dec256  => Dec128,

}
