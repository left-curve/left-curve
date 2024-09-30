use {
    crate::{Dec, Int, NumberConst},
    bnum::types::{I256, I512, U256, U512},
};

pub trait IsZero {
    fn is_zero(&self) -> bool;
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

impl<U> IsZero for Dec<U>
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

// ------------------------------------ tests ------------------------------------

#[cfg(test)]
mod tests {
    use crate::{int_test, test_utils::dt, Int, IsZero, NumberConst};

    int_test!( is_zero
        method = |zero: Int<_>| {
            assert!(zero.is_zero());
            let non_zero = Int::ONE;
            dt(non_zero, zero);
            assert!(!non_zero.is_zero());
        }
    );
}
