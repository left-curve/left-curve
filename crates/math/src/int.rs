use {
    crate::{
        utils::{bytes_to_digits, grow_le_int, grow_le_uint},
        Integer, MathError, MathResult, NextNumber, Number,
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
pub struct Int<U>(pub(crate) U);

impl<U> Int<U> {
    pub const fn new(value: U) -> Self {
        Self(value)
    }
}

impl<U> Int<U>
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

impl<U> Int<U>
where
    Int<U>: NextNumber,
    <Int<U> as NextNumber>::Next: Number,
{
    pub fn checked_full_mul(
        self,
        rhs: impl Into<Self>,
    ) -> MathResult<<Int<U> as NextNumber>::Next> {
        let s = self.into_next();
        let r = rhs.into().into_next();
        s.checked_mul(r)
    }
}

impl<U> FromStr for Int<U>
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

impl<U> fmt::Display for Int<U>
where
    U: Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<U> ser::Serialize for Int<U>
where
    Int<U>: Display,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de, U> de::Deserialize<'de> for Int<U>
where
    Int<U>: FromStr,
    <Int<U> as FromStr>::Err: Display,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_str(IntVisitor::<U>::new())
    }
}

struct IntVisitor<U> {
    _marker: PhantomData<U>,
}

impl<U> IntVisitor<U> {
    pub fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

impl<'de, U> de::Visitor<'de> for IntVisitor<U>
where
    Int<U>: FromStr,
    <Int<U> as FromStr>::Err: Display,
{
    type Value = Int<U>;

    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("a string-encoded unsigned integer")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Int::<U>::from_str(v).map_err(E::custom)
    }
}

impl<U> Neg for Int<U>
where
    U: Neg<Output = U>,
{
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self(-self.0)
    }
}

impl<U> Add for Int<U>
where
    U: Number,
{
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        self.checked_add(rhs).unwrap_or_else(|err| panic!("{err}"))
    }
}

impl<U> Sub for Int<U>
where
    U: Number,
{
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        self.checked_sub(rhs).unwrap_or_else(|err| panic!("{err}"))
    }
}

impl<U> Mul for Int<U>
where
    U: Number,
{
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        self.checked_mul(rhs).unwrap_or_else(|err| panic!("{err}"))
    }
}

impl<U> Div for Int<U>
where
    U: Number,
{
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        self.checked_div(rhs).unwrap_or_else(|err| panic!("{err}"))
    }
}

impl<U> Rem for Int<U>
where
    U: Number,
{
    type Output = Self;

    fn rem(self, rhs: Self) -> Self::Output {
        self.checked_rem(rhs).unwrap_or_else(|err| panic!("{err}"))
    }
}

impl<U> Shl<u32> for Int<U>
where
    U: Integer,
{
    type Output = Self;

    fn shl(self, rhs: u32) -> Self::Output {
        self.checked_shl(rhs).unwrap_or_else(|err| panic!("{err}"))
    }
}

impl<U> Shr<u32> for Int<U>
where
    U: Integer,
{
    type Output = Self;

    fn shr(self, rhs: u32) -> Self::Output {
        self.checked_shr(rhs).unwrap_or_else(|err| panic!("{err}"))
    }
}

impl<U> AddAssign for Int<U>
where
    U: Number + Copy,
{
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl<U> SubAssign for Int<U>
where
    U: Number + Copy,
{
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl<U> MulAssign for Int<U>
where
    U: Number + Copy,
{
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}

impl<U> DivAssign for Int<U>
where
    U: Number + Copy,
{
    fn div_assign(&mut self, rhs: Self) {
        *self = *self / rhs;
    }
}

impl<U> RemAssign for Int<U>
where
    U: Number + Copy,
{
    fn rem_assign(&mut self, rhs: Self) {
        *self = *self % rhs;
    }
}

impl<U> ShlAssign<u32> for Int<U>
where
    U: Integer + Copy,
{
    fn shl_assign(&mut self, rhs: u32) {
        *self = *self << rhs;
    }
}

impl<U> ShrAssign<u32> for Int<U>
where
    U: Integer + Copy,
{
    fn shr_assign(&mut self, rhs: u32) {
        *self = *self >> rhs;
    }
}

// ------------------------------ concrete types -------------------------------

/// 64-bit unsigned integer.
pub type Uint64 = Int<u64>;

/// 128-bit unsigned integer.
pub type Uint128 = Int<u128>;

/// 256-bit unsigned integer.
pub type Uint256 = Int<U256>;

/// 512-bit unsigned integer.
pub type Uint512 = Int<U512>;

/// 64-bit signed integer.
pub type Int64 = Int<i64>;

/// 128-bit signed integer.
pub type Int128 = Int<i128>;

/// 256-bit signed integer.
pub type Int256 = Int<I256>;

/// 512-bit signed integer.
pub type Int512 = Int<I512>;

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
    use {super::*, crate::Inner, proptest::prelude::*};

    proptest! {
        #[test]
        fn uint256_const_constructor(input in any::<u128>()) {
            let uint256 = Uint256::new_from_u128(input);
            let output = uint256.into_inner().try_into().unwrap();
            assert_eq!(input, output);
        }

        #[test]
        fn uint512_const_constructor(input in any::<u128>()) {
            let uint512 = Uint512::new_from_u128(input);
            let output = uint512.into_inner().try_into().unwrap();
            assert_eq!(input, output);
        }

        fn int256_const_constructor(input in any::<i128>()) {
            let int256 = Int256::new_from_i128(input);
            let output = int256.into_inner().try_into().unwrap();
            assert_eq!(input, output);
        }
        fn int512_const_constructor(input in any::<i128>()) {
            let int512 = Int512::new_from_i128(input);
            let output = int512.into_inner().try_into().unwrap();
            assert_eq!(input, output);
        }
    }

    #[test]
    fn signed_from_str() {
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
