use {
    crate::{
        Dec128, Dec256, FixedPoint, Int128, Int256, Int512, Int64, MathError, MathResult, Number,
        NumberConst, Sign, Udec128, Udec256, Uint128, Uint256, Uint512, Uint64,
    },
    bnum::cast::As,
};

// -------------------------------- int -> dec ---------------------------------

macro_rules! impl_checked_into_dec {
    ($int:ty => $dec:ty) => {
        impl $int {
            pub fn checked_into_dec(self) -> MathResult<$dec> {
                self.checked_mul(<$dec>::DECIMAL_FRACTION).map(<$dec>::raw)
            }
        }
    };
    ($($int:ty => $dec:ty),+ $(,)?) => {
        $(
            impl_checked_into_dec!($int => $dec);
        )+
    };
}

impl_checked_into_dec! {
    Uint128 => Udec128,
    Uint256 => Udec256,
     Int128 =>  Dec128,
     Int256 =>  Dec256,
}

// -------------------------------- dec -> int ---------------------------------

macro_rules! impl_into_int {
    ($dec:ty => $int:ty) => {
        impl $dec {
            pub fn into_int(self) -> $int {
                // The decimal fraction is non-zero, so safe to unwrap.
                self.0.checked_div(<$dec>::DECIMAL_FRACTION).unwrap()
            }
        }
    };
    ($($dec:ty => $int:ty),+ $(,)?) => {
        $(
            impl_into_int!($dec => $int);
        )+
    };
}

impl_into_int! {
    Udec128 => Uint128,
    Udec256 => Uint256,
     Dec128 =>  Int128,
     Dec256 =>  Int256,
}

// ---------------------------- unsigned -> signed -----------------------------

macro_rules! impl_checked_into_signed_std {
    ($unsigned:ty => $signed:ty) => {
        impl $unsigned {
            pub fn checked_into_signed(self) -> MathResult<$signed> {
                if self.0 > <$signed>::MAX.0 as _ {
                    return Err(MathError::overflow_conversion::<$unsigned, $signed>(self));
                }

                Ok(<$signed>::new(self.0 as _))
            }
        }
    };
    ($($unsigned:ty => $signed:ty),+ $(,)?) => {
        $(
            impl_checked_into_signed_std!($unsigned => $signed);
        )+
    };
}

impl_checked_into_signed_std! {
    Uint64  => Int64,
    Uint128 => Int128,
}

macro_rules! impl_checked_into_signed_bnum {
    ($unsigned:ty => $signed:ty) => {
        impl $unsigned {
            pub fn checked_into_signed(self) -> MathResult<$signed> {
                if self.0 > <$signed>::MAX.0.as_() {
                    return Err(MathError::overflow_conversion::<$unsigned, $signed>(self));
                }

                Ok(<$signed>::new(self.0.as_()))
            }
        }
    };
    ($($unsigned:ty => $signed:ty),+ $(,)?) => {
        $(
            impl_checked_into_signed_bnum!($unsigned => $signed);
        )+
    };
}

impl_checked_into_signed_bnum! {
    Uint256 => Int256,
    Uint512 => Int512,
}

macro_rules! impl_chekced_into_signed_dec {
    ($unsigned:ty => $signed:ty) => {
        impl $unsigned {
            pub fn checked_into_signed(self) -> MathResult<$signed> {
                self.0.checked_into_signed().map(<$signed>::raw)
            }
        }
    };
    ($($unsigned:ty => $signed:ty),+ $(,)?) => {
        $(
            impl_chekced_into_signed_dec!($unsigned => $signed);
        )+
    };
}

impl_chekced_into_signed_dec! {
    Udec128 => Dec128,
    Udec256 => Dec256,
}

// ---------------------------- signed -> unsigned -----------------------------

macro_rules! impl_checked_into_unsigned_std {
    ($signed:ty => $unsigned:ty) => {
        impl $signed {
            pub fn checked_into_unsigned(self) -> MathResult<$unsigned> {
                if self.is_negative() {
                    return Err(MathError::overflow_conversion::<$signed, $unsigned>(self));
                }

                Ok(<$unsigned>::new(self.0 as _))
            }
        }
    };
    ($($signed:ty => $unsigned:ty),+ $(,)?) => {
        $(
            impl_checked_into_unsigned_std!($signed => $unsigned);
        )+
    };
}

impl_checked_into_unsigned_std! {
    Int64  => Uint64,
    Int128 => Uint128,
}

macro_rules! impl_checked_into_unsigned_bnum {
    ($signed:ty => $unsigned:ty) => {
        impl $signed {
            pub fn checked_into_unsigned(self) -> MathResult<$unsigned> {
                if self.is_negative() {
                    return Err(MathError::overflow_conversion::<$signed, $unsigned>(self));
                }

                Ok(<$unsigned>::new(self.0.as_()))
            }
        }
    };
    ($($signed:ty => $unsigned:ty),+ $(,)?) => {
        $(
            impl_checked_into_unsigned_bnum!($signed => $unsigned);
        )+
    };
}

impl_checked_into_unsigned_bnum! {
    Int256 => Uint256,
    Int512 => Uint512,
}

macro_rules! impl_checked_into_unsigned_dec {
    ($signed:ty => $unsigned:ty) => {
        impl $signed {
            pub fn checked_into_unsigned(self) -> MathResult<$unsigned> {
                self.0.checked_into_unsigned().map(<$unsigned>::raw)
            }
        }
    };
    ($($signed:ty => $unsigned:ty),+ $(,)?) => {
        $(
            impl_checked_into_unsigned_dec!($signed => $unsigned);
        )+
    };
}

impl_checked_into_unsigned_dec! {
    Dec128 => Udec128,
    Dec256 => Udec256,
}
