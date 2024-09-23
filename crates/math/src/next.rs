use crate::{Int128, Int256, Int64, Uint128, Uint256, Uint512, Uint64};

/// Describes a number type can be cast into another type of a bigger word size.
///
/// For example, [`Uint128`](crate::Uint128) can be safety cast to [`Uint256`](crate::Uint256).
/// In this case, [`NextNumber`] trait should be implemented for [`Uint128`](crate::Uint128)
/// with `Next` being [`Uint256`](crate::Uint256).
pub trait NextNumber: Sized + TryFrom<Self::Next> {
    type Next: From<Self>;

    fn into_next(self) -> Self::Next {
        self.into()
    }
}

macro_rules! impl_next {
    ($this:ty => $next:ty) => {
        impl NextNumber for $this {
            type Next = $next;
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
    Uint256 => Uint512,
    Int64   => Int128,
    Int128  => Int256,

    // TODO: Fix impl From<I256> for I512 in bnum
    // Int256  => Int512,
}
