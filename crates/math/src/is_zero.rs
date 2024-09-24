use {
    crate::{Dec, Int, NumberConst},
    bnum::types::{I256, I512, U256, U512},
};

pub trait IsZero {
    fn is_zero(&self) -> bool;
}

// ----------------------------------- uint ------------------------------------

impl<U> IsZero for Int<U>
where
    U: IsZero,
{
    fn is_zero(&self) -> bool {
        self.0.is_zero()
    }
}

// ----------------------------------- udec ------------------------------------

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
