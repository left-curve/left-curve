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

#[cfg(test)]
mod tests {
    use {
        super::NextNumber,
        crate::{int_test, test_utils::bt, Int},
        bnum::{
            cast::As,
            types::{I256, I512, U256, U512},
        },
    };

    int_test!( next
        inputs = {
            u128 = {
                passing: [
                    (u128::MAX, U256::from(u128::MAX))
                ]
            }
            u256 = {
                passing: [
                    (U256::MAX, U256::MAX.as_::<U512>())
                ]
            }
            i128 = {
                passing: [
                    (i128::MAX, I256::from(i128::MAX)),
                    (i128::MIN, I256::from(i128::MIN))
                ]
            }
            i256 = {
                passing: [
                    (I256::MAX, I256::MAX.as_::<I512>()),
                    (I256::MIN, I256::MIN.as_::<I512>())
                ]
            }
        }
        method = |_0, samples| {
            for (current, next) in samples {
                let current = bt(_0, Int::new(current));
                assert_eq!(current.into_next(), Int::new(next));
            }
        }
    );
}
