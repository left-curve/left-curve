use {
    crate::{
        utils::{bytes_to_digits, grow_le_int, grow_le_uint},
        Inner, Integer, MathError, MathResult, NextNumber, Number,
    },
    bnum::types::{I256, I512, U256, U512},
    borsh::{BorshDeserialize, BorshSerialize},
    serde::{de, ser},
    std::{
        fmt::{self, Display},
        marker::PhantomData,
        ops::{
            Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Rem, RemAssign, Shl, ShlAssign,
            Shr, ShrAssign, Sub, SubAssign,
        },
        str::FromStr,
    },
};

// ------------------------------- generic type --------------------------------

#[derive(
    BorshSerialize, BorshDeserialize, Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord,
)]
pub struct Uint<U>(pub(crate) U);

impl<U> Uint<U> {
    pub const fn new(value: U) -> Self {
        Self(value)
    }
}

impl<U> Uint<U>
where
    U: Copy,
{
    pub const fn number(&self) -> U {
        self.0
    }

    pub const fn number_ref(&self) -> &U {
        &self.0
    }
}

impl<U> Uint<U>
where
    Uint<U>: NextNumber,
    <Uint<U> as NextNumber>::Next: Number,
{
    pub fn checked_full_mul(
        self,
        rhs: impl Into<Self>,
    ) -> MathResult<<Uint<U> as NextNumber>::Next> {
        let s = self.into_next();
        let r = rhs.into().into_next();
        s.checked_mul(r)
    }
}

impl<U> FromStr for Uint<U>
where
    U: FromStr,
    <U as FromStr>::Err: ToString,
{
    type Err = MathError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        U::from_str(s)
            .map(Self)
            .map_err(|err| MathError::parse_number::<Self, _, _>(s, err))
    }
}

impl<U> fmt::Display for Uint<U>
where
    U: Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<U> ser::Serialize for Uint<U>
where
    Uint<U>: Display,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de, U> de::Deserialize<'de> for Uint<U>
where
    Uint<U>: FromStr,
    <Uint<U> as FromStr>::Err: Display,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_str(UintVisitor::<U>::new())
    }
}

struct UintVisitor<U> {
    _marker: PhantomData<U>,
}

impl<U> UintVisitor<U> {
    pub fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

impl<'de, U> de::Visitor<'de> for UintVisitor<U>
where
    Uint<U>: FromStr,
    <Uint<U> as FromStr>::Err: Display,
{
    type Value = Uint<U>;

    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("a string-encoded unsigned integer")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Uint::<U>::from_str(v).map_err(E::custom)
    }
}

impl<U> Neg for Uint<U>
where
    U: Neg<Output = U>,
{
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self(-self.0)
    }
}

impl<U> Add for Uint<U>
where
    U: Number,
{
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        self.checked_add(rhs).unwrap_or_else(|err| panic!("{err}"))
    }
}

impl<U> Sub for Uint<U>
where
    U: Number,
{
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        self.checked_sub(rhs).unwrap_or_else(|err| panic!("{err}"))
    }
}

impl<U> Mul for Uint<U>
where
    U: Number,
{
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        self.checked_mul(rhs).unwrap_or_else(|err| panic!("{err}"))
    }
}

impl<U> Div for Uint<U>
where
    U: Number,
{
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        self.checked_div(rhs).unwrap_or_else(|err| panic!("{err}"))
    }
}

impl<U> Rem for Uint<U>
where
    U: Number,
{
    type Output = Self;

    fn rem(self, rhs: Self) -> Self::Output {
        self.checked_rem(rhs).unwrap_or_else(|err| panic!("{err}"))
    }
}

impl<U> Shl<u32> for Uint<U>
where
    U: Integer,
{
    type Output = Self;

    fn shl(self, rhs: u32) -> Self::Output {
        self.checked_shl(rhs).unwrap_or_else(|err| panic!("{err}"))
    }
}

impl<U> Shr<u32> for Uint<U>
where
    U: Integer,
{
    type Output = Self;

    fn shr(self, rhs: u32) -> Self::Output {
        self.checked_shr(rhs).unwrap_or_else(|err| panic!("{err}"))
    }
}

impl<U> AddAssign for Uint<U>
where
    U: Number + Copy,
{
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl<U> SubAssign for Uint<U>
where
    U: Number + Copy,
{
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl<U> MulAssign for Uint<U>
where
    U: Number + Copy,
{
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}

impl<U> DivAssign for Uint<U>
where
    U: Number + Copy,
{
    fn div_assign(&mut self, rhs: Self) {
        *self = *self / rhs;
    }
}

impl<U> RemAssign for Uint<U>
where
    U: Number + Copy,
{
    fn rem_assign(&mut self, rhs: Self) {
        *self = *self % rhs;
    }
}

impl<U> ShlAssign<u32> for Uint<U>
where
    U: Integer + Copy,
{
    fn shl_assign(&mut self, rhs: u32) {
        *self = *self << rhs;
    }
}

impl<U> ShrAssign<u32> for Uint<U>
where
    U: Integer + Copy,
{
    fn shr_assign(&mut self, rhs: u32) {
        *self = *self >> rhs;
    }
}

// ------------------------------ concrete types -------------------------------

macro_rules! generate_uint {
    (
        name       = $name:ident,
        inner_type = $inner:ty,
        from_int   = [$($from:ty),*],
        from_std   = [$($from_std:ty),*],
        doc        = $doc:literal,
    ) => {
        #[doc = $doc]
        pub type $name = Uint<$inner>;

        // --- Impl From Uint and from inner type ---
        $(
            // Ex: From<Uint64> for Uint128
            impl From<$from> for $name {
                fn from(value: $from) -> Self {
                    Self(value.number().into())
                }
            }

            // Ex: From<u64> for Uint128
            impl From<<$from as Inner>::U> for $name {
                fn from(value: <$from as Inner>::U) -> Self {
                    Self(value.into())
                }
            }

            // Ex: TryFrom<Uint128> for Uint64
            impl TryFrom<$name> for $from {
                type Error = MathError;
                fn try_from(value: $name) -> MathResult<$from> {
                    <$from as Inner>::U::try_from(value.number())
                        .map(Self)
                        .map_err(|_| MathError::overflow_conversion::<_, $from>(value))
                }
            }

            // Ex: TryFrom<Uint128> for u64
            impl TryFrom<$name> for <$from as Inner>::U {
                type Error = MathError;
                fn try_from(value: $name) -> MathResult<<$from as Inner>::U> {
                    <$from as Inner>::U::try_from(value.number())
                        .map_err(|_| MathError::overflow_conversion::<_, $from>(value))
                }
            }
        )*

        // --- Impl From std ---
        $(
            // Ex: From<u32> for Uint128
            impl From<$from_std> for $name {
                fn from(value: $from_std) -> Self {
                    Self(value.into())
                }
            }

            // Ex: TryFrom<Uint128> for u32
            impl TryFrom<$name> for $from_std {
                type Error = MathError;
                fn try_from(value: $name) -> MathResult<$from_std> {
                    <$from_std>::try_from(value.number())
                    .map_err(|_| MathError::overflow_conversion::<_, $from_std>(value))
                }
            }
        )*

        // Ex: From<u128> for Uint128
        impl From<$inner> for $name {
            fn from(value: $inner) -> Self {
                Self::new(value)
            }
        }

        // Ex: From<Uint128> for u128
        impl From<$name> for $inner {
            fn from(value: $name) -> Self {
               value.number()
            }
        }
    };
}

generate_uint! {
    name       = Uint64,
    inner_type = u64,
    from_int   = [],
    from_std   = [u32, u16, u8],
    doc        = "64-bit unsigned integer.",
}

generate_uint! {
    name       = Uint128,
    inner_type = u128,
    from_int   = [Uint64],
    from_std   = [u32, u16, u8],
    doc        = "128-bit unsigned integer.",
}

generate_uint! {
    name       = Uint256,
    inner_type = U256,
    from_int   = [Uint64, Uint128],
    from_std   = [u32, u16, u8],
    doc        = "256-bit unsigned integer.",
}

generate_uint! {
    name       = Uint512,
    inner_type = U512,
    from_int   = [Uint256, Uint64, Uint128],
    from_std   = [u32, u16, u8],
    doc        = "512-bit unsigned integer.",
}

generate_uint! {
    name = Int64,
    inner_type = i64,
    from_int = [],
    from_std = [u32, u16, u8],
    doc = "64-bit signed integer.",
}

generate_uint! {
    name = Int128,
    inner_type = i128,
    from_int = [Int64, Uint64],
    from_std = [u32, u16, u8],
    doc = "128-bit signed integer.",
}

generate_uint! {
    name = Int256,
    inner_type = I256,
    from_int = [Int128, Int64, Uint128, Uint64],
    from_std = [u32, u16, u8],
    doc = "256-bit signed integer.",
}

generate_uint! {
    name = Int512,
    inner_type = I512,
    from_int = [Int128, Int64, Uint128, Uint64],
    from_std = [u32, u16, u8],
    doc = "512-bit signed integer.",
}

// -------------- additional constructor methods for Uint256/512 & Int256/512 ---------------

impl Uint256 {
    pub const fn new_from_u128(value: u128) -> Self {
        let grown_bytes = grow_le_uint::<16, 32>(value.to_le_bytes());
        let digits = bytes_to_digits(grown_bytes);
        Self(U256::from_digits(digits))
    }
}

impl Uint512 {
    pub const fn new_from_u128(value: u128) -> Self {
        let grown_bytes = grow_le_uint::<16, 64>(value.to_le_bytes());
        let digits = bytes_to_digits(grown_bytes);
        Self(U512::from_digits(digits))
    }
}

impl Int256 {
    pub const fn new_from_i128(value: i128) -> Self {
        let grown_bytes = grow_le_int::<16, 32>(value.to_le_bytes());
        let digits = bytes_to_digits(grown_bytes);
        Self(I256::from_bits(U256::from_digits(digits)))
    }
}

impl Int512 {
    pub const fn new_from_i128(value: i128) -> Self {
        let grown_bytes = grow_le_int::<16, 64>(value.to_le_bytes());
        let digits = bytes_to_digits(grown_bytes);
        Self(I512::from_bits(U512::from_digits(digits)))
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {super::*, proptest::prelude::*};

    proptest! {
        #[test]
        fn uint256_const_constructor(input in any::<u128>()) {
            let uint256 = Uint256::new_from_u128(input);
            let output = uint256.number().try_into().unwrap();
            assert_eq!(input, output);
        }

        #[test]
        fn uint512_const_constructor(input in any::<u128>()) {
            let uint512 = Uint512::new_from_u128(input);
            let output = uint512.number().try_into().unwrap();
            assert_eq!(input, output);
        }

        fn int256_const_constructor(input in any::<i128>()) {
            let int256 = Int256::new_from_i128(input);
            let output = int256.number().try_into().unwrap();
            assert_eq!(input, output);
        }
        fn int512_const_constructor(input in any::<i128>()) {
            let int512 = Int512::new_from_i128(input);
            let output = int512.number().try_into().unwrap();
            assert_eq!(input, output);
        }
    }

    #[test]
    fn singed_from_str() {
        assert_eq!(Int128::from_str("100").unwrap(), Int128::new(100));
        assert_eq!(Int128::from_str("-100").unwrap(), Int128::new(-100));
        assert_eq!(
            Int512::from_str("100").unwrap(),
            Int512::new(I512::from(100))
        );
        assert_eq!(
            Int512::from_str("-100").unwrap(),
            Int512::new(I512::from(-100))
        );
    }

    #[test]
    fn neg_works() {
        assert_eq!(-Int512::new_from_i128(-100), Int512::new(I512::from(100)));
        assert_eq!(-Int512::new_from_i128(100), Int512::new(I512::from(-100)))
    }
}
