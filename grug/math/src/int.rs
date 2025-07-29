use {
    crate::{
        Integer, MathError, MathResult, NextNumber, Number, NumberConst,
        utils::{bytes_to_digits, grow_le_int, grow_le_uint},
    },
    bnum::types::{I256, I512, U256, U512},
    borsh::{BorshDeserialize, BorshSerialize},
    serde::{de, ser},
    std::{
        fmt::{self, Display},
        iter::Sum,
        marker::PhantomData,
        ops::{
            Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Not, Rem, RemAssign, Shl,
            ShlAssign, Shr, ShrAssign, Sub, SubAssign,
        },
        str::FromStr,
    },
};

// ------------------------------- generic type --------------------------------

#[derive(
    BorshSerialize,
    BorshDeserialize,
    Default,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
)]
pub struct Int<U>(pub U);

impl<U> Int<U> {
    pub const fn new(value: U) -> Self {
        Self(value)
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

impl<U> de::Visitor<'_> for IntVisitor<U>
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

impl<U> Not for Int<U>
where
    U: Not<Output = U>,
{
    type Output = Self;

    fn not(self) -> Self::Output {
        Self(!self.0)
    }
}

impl<U> Sum for Int<U>
where
    U: Number + NumberConst,
{
    fn sum<I>(iter: I) -> Self
    where
        I: Iterator<Item = Self>,
    {
        let mut sum = Self::ZERO;
        for int in iter {
            sum += int;
        }
        sum
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

// ---------------------- additional constructor methods -----------------------

impl From<u64> for Uint64 {
    fn from(value: u64) -> Self {
        Self::new(value)
    }
}

impl From<u128> for Uint128 {
    fn from(value: u128) -> Self {
        Self::new(value)
    }
}

impl From<i64> for Int64 {
    fn from(value: i64) -> Self {
        Self::new(value)
    }
}

impl From<i128> for Int128 {
    fn from(value: i128) -> Self {
        Self::new(value)
    }
}

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
pub mod tests {
    use {
        super::*,
        crate::{
            NumberConst, dts, int_test,
            test_utils::{bt, dt},
        },
        bnum::cast::As,
    };

    int_test!( size_of
        inputs = {
            u128 = [16]
            u256 = [32]
            i128 = [16]
            i256 = [32]
        }
        method = |_0, size| {
            assert_eq!(core::mem::size_of_val(&_0), size);
        }
    );

    int_test!( from_str
        inputs = {
            u128 = {
                passing: [
                    (128_u128, "128"),
                ]
            }
            u256 = {
                passing: [
                    (U256::from(256_u128), "256"),
                ]
            }
            i128 = {
                passing: [
                    (-128_i128, "-128"), (-128_i128, "-128"),
                ]
            }
            i256 = {
                passing: [
                    (I256::from(256_i128), "256"), (I256::from(-256_i128), "-256"),
                ]
            }
        }
        method = |_, samples| {
            for (val, str) in samples {
                let original = Int::new(val);
                assert_eq!(original.0, val);

                let from_str = Int::from_str(str).unwrap();
                assert_eq!(from_str, original);

                let as_into = original.0;
                dt(as_into, val);
                assert_eq!(as_into, val);
            }
        }
    );

    int_test!( display
        inputs = {
            u128 = {
                passing: [
                    (Uint128::new(128_u128), "128"),
                ]
            }
            u256 = {
                passing: [
                    (Uint256::new(U256::from(256_u128)), "256"),
                ]
            }
            i128 = {
                passing: [
                    (Int128::MAX, "170141183460469231731687303715884105727"),
                    (Int128::MIN, "-170141183460469231731687303715884105728"),
                    (Int128::new(i128::ZERO), "0"),
                ]
            }
            i256 = {
                passing: [
                    (Int256::MAX, "57896044618658097711785492504343953926634992332820282019728792003956564819967"),
                    (Int256::MIN, "-57896044618658097711785492504343953926634992332820282019728792003956564819968"),
                    (Int256::ZERO, "0"),
                ]
            }
        }
        method = |_, samples| {
            for (number, str) in samples {
                assert_eq!(format!("{number}"), str);
            }
        }
    );

    int_test!( display_padding_front
        inputs = {
            u128 = {
                passing: [
                    ("00128", "128"),
                ]
            }
            u256 = {
                passing: [
                    ("000256", "256"),
                ]
            }
            i128 = {
                passing: [
                    ("000128", "128"),
                    ("-000128", "-128"),
                ]
            }
            i256 = {
                passing: [
                    ("000256", "256"),
                    ("-000256", "-256"),
                ]
            }
        }
        method = |_0, samples| {
            for (padded_str, compare) in samples {
                let uint = bt(_0, Int::from_str(padded_str).unwrap());
                assert_eq!(format!("{uint}"), compare);
            }
        }
    );

    int_test!( json
        inputs = {
            u128 = {
                passing: ["123456"]
            }
            u256 = {
                passing: ["123456"]
            }
            i128 = {
                passing: ["123456", "-123456"]
            }
            i256 = {
                passing: ["123456", "-123456"]
            }
        }
        method = |_0, samples| {
            for sample in samples {
                let original = bt(_0, Int::from_str(sample).unwrap());

                let serialized_str = serde_json::to_string(&original).unwrap();
                assert_eq!(serialized_str, format!("\"{sample}\""));

                let serialized_vec = serde_json::to_vec(&original).unwrap();
                assert_eq!(serialized_vec, format!("\"{sample}\"").as_bytes());

                let parsed: Int::<_> = serde_json::from_str(&serialized_str).unwrap();
                assert_eq!(parsed, original);

                let parsed: Int::<_> = serde_json::from_slice(&serialized_vec).unwrap();
                assert_eq!(parsed, original);
            }
        }
    );

    int_test!( compare
        inputs = {
            u128 = {
                passing: [
                    (10_u128, 200_u128),
                ]
            }
            u256 = {
                passing: [
                    (U256::from(10_u128), U256::from(200_u128)),
                ]
            }
            i128 = {
                passing: [
                    (10_i128, 200_i128),
                    (-10, 200),
                    (-200, -10),
                ]
            }
            i256 = {
                passing: [
                    (I256::from(10), I256::from(200_i128)),
                    (I256::from(-10), I256::from(200)),
                    (I256::from(-200), I256::from(-10)),
                ]
            }
        }
        method = |_0, samples| {
            for (low, high) in samples {
                let low = Int::new(low);
                let high = Int::new(high);
                dts!(_0, low, high);
                assert!(low < high);
                assert!(high > low);
                assert_eq!(low, low);
            }
        }

    );

    int_test!( partial_eq
        inputs = {
            u128 = {
                passing: [
                    (1_u128, 1_u128),
                    (42_u128, 42_u128),
                    (u128::MAX, u128::MAX),
                    (0_u128, 0_u128),
                ],
                failing: [
                    (42_u128, 24_u128),
                    (24_u128, 42_u128),
                ]
            }
            u256 = {
                passing: [
                    (U256::from(1_u128), U256::from(1_u128)),
                    (U256::from(42_u128), U256::from(42_u128)),
                    (U256::from(u128::MAX), U256::from(u128::MAX)),
                    (U256::from(0_u128), U256::from(0_u128)),
                ],
                failing: [
                    (U256::from(42_u128), U256::from(24_u128)),
                    (U256::from(24_u128), U256::from(42_u128)),
                ]
            }
            i128 = {
                passing: [
                    (1_i128, 1_i128),
                    (42_i128, 42_i128),
                    (i128::MAX, i128::MAX),
                    (0_i128, 0_i128),
                    (-42_i128, -42_i128),
                ],
                failing: [
                    (42_i128, 24_i128),
                    (24_i128, 42_i128),
                    (-42_i128, 42_i128),
                    (42_i128, -42_i128),
                    (i128::MIN, -i128::MAX),
                ]
            }
            i256 = {
                passing: [
                    (I256::from(1_i128), I256::from(1_i128)),
                    (I256::from(42_i128), I256::from(42_i128)),
                    (I256::from(i128::MAX), I256::from(i128::MAX)),
                    (I256::from(0_i128), I256::from(0_i128)),
                    (I256::from(-42_i128), I256::from(-42_i128))
                ]
                failing: [
                    (I256::from(42_i128), I256::from(24_i128)),
                    (I256::from(24_i128), I256::from(42_i128)),
                    (I256::from(-42_i128), I256::from(24_i128)),
                    (I256::from(42_i128), I256::from(-24_i128)),
                    (I256::MIN, -I256::MAX),
                ]
            }
        }
        method = |_0, passing, failing| {
            for (lhs, rhs) in passing {
                let lhs = Int::new(lhs);
                let rhs = Int::new(rhs);
                assert!(lhs == rhs);
            }

            for (lhs, rhs) in failing {
                let lhs = Int::new(lhs);
                let rhs = Int::new(rhs);
                assert!(lhs != rhs);
            }
        }
    );

    int_test!( neg
        inputs = {
            i128 = {
                passing: [
                    (0_i128, 0_i128),
                    (42_i128, -42_i128),
                    (-42_i128, 42_i128),
                    (i128::MAX, i128::MIN + 1),
                    (i128::MIN + 1, i128::MAX),
                ]
            }
            i256 = {
                passing: [
                    (I256::from(0_i128), I256::from(0_i128)),
                    (I256::from(42_i128), I256::from(-42_i128)),
                    (I256::from(-42_i128), I256::from(42_i128)),
                    (I256::MAX, I256::MIN + I256::from(1)),
                    (I256::MIN + I256::from(1), I256::MAX),
                ]
            }
        }
        method = |_0, passing| {
            for (input, expected) in passing {
                let input = Int::new(input);
                let expected = Int::new(expected);
                assert_eq!(-input, expected);
            }
        }
    );

    int_test!( checked_full_mul
        inputs = {
            u128 = {
                passing: [
                    (u128::MAX, 2_u128, Uint256::new(U256::from(u128::MAX) * U256::from(2_u128))),
                    (u128::TEN, 10_u128, Uint256::new(U256::from(100_u128))),
                ]
            }
            u256 = {
                passing: [
                    (U256::MAX, U256::from(2_u128), Uint512::new((U256::MAX).as_::<U512>() * U512::from(2_u128))),
                    (U256::TEN, U256::from(10_u128), Uint512::new(U512::from(100_u128))),
                ]
            }
            i128 = {
                passing: [
                    (i128::MAX, 2_i128, Int256::new(I256::from(i128::MAX) * I256::from(2))),
                    (i128::TEN, 10_i128, Int256::new(I256::from(100))),
                    (i128::MIN, 10_i128, Int256::new(I256::from(i128::MIN) * I256::from(10))),
                    (i128::MIN, -10_i128, Int256::new(I256::from(i128::MIN) * I256::from(-10))),
                    (i128::MAX, -10_i128, Int256::new(I256::from(i128::MAX) * I256::from(-10))),
                ]
            }
            i256 = {
                passing: [
                    (I256::MAX, I256::from(2_i128), Int512::new((I256::MAX).as_::<I512>() * I512::from(2))),
                    (I256::TEN, I256::from(10_i128), Int512::new(I512::from(100))),
                    (I256::MIN, I256::from(10_i128), Int512::new((I256::MIN).as_::<I512>() * I512::from(10))),
                    (I256::MIN, I256::from(-10_i128), Int512::new((I256::MIN).as_::<I512>() * I512::from(-10))),
                    (I256::MAX, I256::from(-10_i128), Int512::new((I256::MAX).as_::<I512>() * I512::from(-10))),
                ]
            }
        }
        method = |_0, passing| {
            for (left, right, expect) in passing {
                let left = bt(_0, Int::new(left));
                let right = bt(_0, Int::new(right));
                assert_eq!(left.checked_full_mul(right).unwrap(), expect);
            }
        }
    );
}
