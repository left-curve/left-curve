use {
    crate::{Bytable, Dec, Int, Int64, Int128, Int256, Int512, Uint64, Uint128, Uint256, Uint512},
    bnum::types::{I512, U512},
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
    ($this:ty => $next_inner:ty => $next:ty) => {
        impl NextNumber for $this {
            type Next = $next;

            fn into_next(self) -> Self::Next {
                <$next>::new(<$next_inner>::from_be_bytes_growing(
                    self.0.to_be_bytes(),
                ))
            }
        }
    };
    ($($this:ty => $next_inner:ty => $next:ty),+ $(,)?) => {
        $(
            impl_next_bnum!($this => $next_inner => $next);
        )+
    };
}

impl_next_bnum! {
    Uint256 => U512 => Uint512,
    Int256  => I512 => Int512,
}

// ----------------------------------- dec -------------------------------------

impl<U, NP, const S: u32> NextNumber for Dec<U, S>
where
    Int<U>: NextNumber<Next = Int<NP>>,
{
    type Next = Dec<NP, S>;

    fn into_next(self) -> Self::Next {
        Dec::raw(self.0.into_next())
    }
}
// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod int_tests {
    use {
        crate::{Int, NextNumber, int_test, test_utils::bt},
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

#[cfg(test)]
mod dec_tests {
    use {
        crate::{
            Dec, Dec128, Dec256, Int256, NextNumber, NumberConst, Udec128, Udec256, Uint256,
            dec_test, test_utils::dt,
        },
        bnum::cast::As,
    };

    dec_test!( next
        inputs = {
            udec128 = {
                passing: [
                    (Udec128::MAX, Udec256::raw(Uint256::new(u128::MAX.as_())))
                ]
            }
            dec128 = {
                passing: [
                    (Dec128::MAX, Dec256::raw(Int256::new(i128::MAX.as_()))),
                    (Dec128::MIN, Dec256::raw(Int256::new(i128::MIN.as_())))
                ]
            }
        }
        method = |_0d:  Dec<_, 18>, samples| {
            for (current, next) in samples {
                // let current = bt(_0, Dec::new(current));
                dt(_0d, current);
                assert_eq!(current.into_next(), next);
            }
        }
    );
}
