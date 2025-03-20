use {
    crate::{Dec, Int, NumberConst},
    bnum::types::{I256, I512, U256, U512},
};

/// Describes a number that can be compared to zero.
pub trait IsZero {
    /// Return true if the number is zero; false otherwise.
    fn is_zero(&self) -> bool;

    /// Return true if the number is not zero; false otherwise.
    #[inline]
    fn is_non_zero(&self) -> bool {
        !self.is_zero()
    }
}

// ------------------------------------ int ------------------------------------

impl<U> IsZero for Int<U>
where
    U: IsZero,
{
    fn is_zero(&self) -> bool {
        self.0.is_zero()
    }
}

// ------------------------------------ dec ------------------------------------

impl<U, const S: u32> IsZero for Dec<U, S>
where
    U: IsZero,
{
    fn is_zero(&self) -> bool {
        self.0.is_zero()
    }
}

// ------------------------------ primitive types ------------------------------

macro_rules! impl_is_zero {
    ($t:ty) => {
        impl IsZero for $t {
            fn is_zero(&self) -> bool {
                *self == Self::ZERO
            }
        }
    };
    ($($t:ty),+ $(,)?) => {
        $(
            impl_is_zero!($t);
        )+
    };
}

impl_is_zero! {
    u8, u16, u32, u64, u128, U256, U512,
    i8, i16, i32, i64, i128, I256, I512,
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod int_tests {
    use crate::{Int, IsZero, NumberConst, int_test, test_utils::bt};

    int_test!( is_zero
        method = |zero: Int<_>| {
            assert!(zero.is_zero());
            assert!(!zero.is_non_zero());

            let non_zero = bt(zero, Int::ONE);
            assert!(!non_zero.is_zero());
            assert!(non_zero.is_non_zero());
        }
    );
}

#[cfg(test)]
mod dec_tests {
    use crate::{Dec, IsZero, NumberConst, dec_test, test_utils::bt};

    dec_test!( is_zero
        method = |zero: Dec<_, 18>| {
            assert!(zero.is_zero());
            assert!(!zero.is_non_zero());

            let non_zero = bt(zero, Dec::ONE);
            assert!(!non_zero.is_zero());
            assert!(non_zero.is_non_zero());
        }
    );
}
