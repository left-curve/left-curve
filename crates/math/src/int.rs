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

macro_rules! generate_int {
    (
        name       = $name:ident,
        inner_type = $inner:ty,
        from_int   = [$($from:ty),*],
        from_std   = [$($from_std:ty),*],
        doc        = $doc:literal,
    ) => {
        #[doc = $doc]
        pub type $name = Int<$inner>;

        // --- Impl From Int and from inner type ---
        $(
            // Ex: From<Uint64> for Uint128
            impl From<$from> for $name {
                fn from(value: $from) -> Self {
                    Self(value.into_inner().into())
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
                    <$from as Inner>::U::try_from(value.into_inner())
                        .map(Self)
                        .map_err(|_| MathError::overflow_conversion::<_, $from>(value))
                }
            }

            // Ex: TryFrom<Uint128> for u64
            impl TryFrom<$name> for <$from as Inner>::U {
                type Error = MathError;
                fn try_from(value: $name) -> MathResult<<$from as Inner>::U> {
                    <$from as Inner>::U::try_from(value.into_inner())
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
                    <$from_std>::try_from(value.into_inner())
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
               value.into_inner()
            }
        }
    };
}

generate_int! {
    name       = Uint64,
    inner_type = u64,
    from_int   = [],
    from_std   = [u32, u16, u8],
    doc        = "64-bit unsigned integer.",
}

generate_int! {
    name       = Uint128,
    inner_type = u128,
    from_int   = [Uint64],
    from_std   = [u32, u16, u8],
    doc        = "128-bit unsigned integer.",
}

generate_int! {
    name       = Uint256,
    inner_type = U256,
    from_int   = [Uint64, Uint128],
    from_std   = [u32, u16, u8],
    doc        = "256-bit unsigned integer.",
}

generate_int! {
    name       = Uint512,
    inner_type = U512,
    from_int   = [Uint256, Uint64, Uint128],
    from_std   = [u32, u16, u8],
    doc        = "512-bit unsigned integer.",
}

generate_int! {
    name       = Int64,
    inner_type = i64,
    from_int   = [],
    from_std   = [u32, u16, u8],
    doc        = "64-bit signed integer.",
}

generate_int! {
    name       = Int128,
    inner_type = i128,
    from_int   = [Int64, Uint64],
    from_std   = [u32, u16, u8],
    doc        = "128-bit signed integer.",
}

generate_int! {
    name       = Int256,
    inner_type = I256,
    from_int   = [Int128, Int64, Uint128, Uint64],
    from_std   = [u32, u16, u8],
    doc        = "256-bit signed integer.",
}

generate_int! {
    name       = Int512,
    inner_type = I512,
    from_int   = [Int128, Int64, Uint128, Uint64],
    from_std   = [u32, u16, u8],
    doc        = "512-bit signed integer.",
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

#[cfg(test)]
pub mod testse {

    use {
        super::*,
        crate::{
            dts, int_test,
            test_utils::{bt, dt, smart_assert},
            Bytable, IsZero, MultiplyFraction, MultiplyRatio, NumberConst, Udec128, Udec256,
        },
    };

    int_test!( size_of,
        Specific
        u128 = [16]
        u256 = [32]
        => |_0, size| {
            assert_eq!(core::mem::size_of_val(&_0), size);
        }
    );

    int_test!( bytable_to_be,
        Specific
        u128 = [&[0u8; 16], &[0xff; 16]]
        u256 = [&[0u8; 32], &[0xff; 32]]
        => |_0, zero_as_byte: &[u8], max_as_byte| {
            let _1 = Int::ONE;
            let max = Int::MAX;
            dts!(_0, _1, max);

            assert_eq!(_0.to_be_bytes().to_vec(), zero_as_byte);

            let mut one_as_bytes: Vec<u8> = zero_as_byte.to_vec();

            // change last byte to 1
            if let Some(last) = one_as_bytes.last_mut() {
                *last = 1u8;
            }
            assert_eq!(_1.to_be_bytes().to_vec(), one_as_bytes);
            assert_eq!(max.to_be_bytes().to_vec(), max_as_byte);
        }
    );

    int_test!( bytable_to_le,
        Specific
        u128 = [&[0u8; 16], &[0xff; 16]]
        u256 = [&[0u8; 32], &[0xff; 32]]
        => |_0, zero_as_byte: &[u8], max_as_byte| {
            let _1 = Int::ONE;
            let max = Int::MAX;
            dts!(_0, _1, max);

            assert_eq!(_0.to_be_bytes().to_vec(), zero_as_byte);

            let mut one_as_bytes: Vec<u8> = zero_as_byte.to_vec();

            // change first byte to 1
            if let Some(first) = one_as_bytes.first_mut() {
                *first = 1u8;
            }
            assert_eq!(_1.to_le_bytes().to_vec(), one_as_bytes);
            assert_eq!(max.to_le_bytes().to_vec(), max_as_byte);
        }
    );

    int_test!( converts,
        Specific
        u128 = [128_u128,             "128"]
        u256 = [U256::from(256_u128), "256"]

        => |_, val, str| {
           let original = Int::new(val);
           assert_eq!(original.0, val);

           let from_str = Int::from_str(str).unwrap();
           assert_eq!(from_str, original);

           let as_into = original.into();
           dt(as_into, val);

           assert_eq!(as_into, val);
        }
    );

    int_test!( from,
        Specific
        u128 = [8_u8, 16_u16, 32_u32, Some(64_u64), None::<u128>]
        u256 = [8_u8, 16_u16, 32_u32, Some(64_u64), Some(128_u128)]
        => |_0, u8, u16, u32, u64, u128| {
            let uint8 = Int::from(u8);
            let uint16 = Int::from(u16);
            let uint32 = Int::from(u32);

            dts!(_0, uint8, uint16, uint32);

            smart_assert(u8, uint8.try_into().unwrap());
            smart_assert(u16, uint16.try_into().unwrap());
            smart_assert(u32, uint32.try_into().unwrap());

            macro_rules! maybe_from {
                ($t:expr) => {
                    if let Some(t) = $t {
                        let uint = bt(_0, Int::from(t));
                        smart_assert(t, uint.try_into().unwrap());
                    }
                };
            }

            maybe_from!(u64);
            maybe_from!(u128);
        }
    );

    int_test!( try_into,
        Specific
       u128 = [Some(Uint256::MAX), Uint256::ZERO, Uint256::from(128_u128), Uint128::from(128_u128)]
       u256 = [Some(Uint512::MAX), Uint512::ZERO, Uint512::from(256_u128), Uint256::from(256_u128)]
       => |_0, next_max, next_zero, next_valid, compare| {

            if let Some(next_max) = next_max {
                let maybe_uint = Int::try_from(next_max);
                dt(&maybe_uint, &Ok(_0));
                maybe_uint.unwrap_err();
            }

            let uint_zero = Int::try_from(next_zero).unwrap();
            assert_eq!(_0, uint_zero);

            let uint = Int::try_from(next_valid).unwrap();
            assert_eq!(uint, compare);

        }
    );

    int_test!( display,
        Specific
        u128 = [Uint128::new(128_u128), "128"]
        u256 = [Uint256::new(U256::from(256_u128)), "256"]
        => |_, uint, str| {
            assert_eq!(format!("{}", uint), str);
        }
    );

    int_test!( display_padding_front,
        Specific
        u128 = ["00128", "128"]
        u256 = ["000256", "256"]
        => |_0, padded_str, compare| {
            let uint = bt(_0, Int::from_str(padded_str).unwrap());
            assert_eq!(format!("{}", uint), compare);
        }
    );

    int_test!( is_zero,
        NoArgs
        => |zero: Int<_>| {
            assert!(zero.is_zero());
            let non_zero = Int::ONE;
            dt(non_zero, zero);
            assert!(!non_zero.is_zero());
        }
    );

    int_test!( json,
        NoArgs
    => |_0| {
        let original = bt(_0, Int::from_str("123456").unwrap());

        let serialized_str = serde_json::to_string(&original).unwrap();
        assert_eq!(serialized_str, format!("\"{}\"", "123456"));

        let serialized_vec = serde_json::to_vec(&original).unwrap();
        assert_eq!(serialized_vec, format!("\"{}\"", "123456").as_bytes());

        let parsed: Int::<_> = serde_json::from_str(&serialized_str).unwrap();
        assert_eq!(parsed, original);

        let parsed: Int::<_> = serde_json::from_slice(&serialized_vec).unwrap();
        assert_eq!(parsed, original);
    });

    int_test!( compare,
        NoArgs
        => |_0| {
            let a = Int::from(10_u64);
            let b = Int::from(20_u64);
            dts!(_0, a, b);

            assert!(a < b);
            assert!(b > a);
            assert_eq!(a, a);
        }
    );

    int_test!( math,
        NoArgs
        attrs = #[allow(clippy::op_ref)]
        => |_0| {
            let a = Int::from(12345_u64);
            let b = Int::from(23456_u64);
            dts!(_0, a, b);

            // test - with
            let diff = bt(_0, Int::from(11111_u64));
            assert_eq!(b - a, diff);

            // test += with
            let mut c = bt(_0, Int::from(300000_u64));
            c += b;
            assert_eq!(c, bt(_0, Int::from(323456_u64)));

            // test -= with
            let mut c = bt(_0, Int::from(300000_u64));
            c -= b;
            assert_eq!(c, bt(_0, Int::from(276544_u64)));


            // error result on underflow (- would produce negative result)
            let underflow_result = a.checked_sub(b);
            let MathError::OverflowSub { .. } = underflow_result.unwrap_err() else {
                panic!("Expected OverflowSub error");
            };
        }
    );

    int_test!( add,
        NoArgs
        attrs = #[allow(clippy::op_ref)]
        => |_0| {
            assert_eq!(
                bt(_0, Int::from(2_u64)) + bt(_0, Int::from(1_u64)),
                bt(_0, Int::from(3_u64))
            );
            assert_eq!(
                bt(_0, Int::from(2_u64)) + bt(_0, Int::from(0_u64)),
                bt(_0, Int::from(2_u64))
            );

            let a = bt(_0, Int::from(10_u64));
            let b = bt(_0, Int::from(3_u64));
            let expected = bt(_0, Int::from(13_u64));
            assert_eq!(a + b, expected);
        }
    );

    int_test!( add_overflow_panics,
        NoArgs
        attrs = #[should_panic(expected = "addition overflow")]
        => |_0| {
            let max = bt(_0, Int::MAX);
            let _ = max + bt(_0, Int::from(12_u64));
        }
    );

    int_test!( sub,
        NoArgs
        attrs = #[allow(clippy::op_ref)]
        => |_0| {
            assert_eq!(bt(_0, Int::from(2_u64)) - bt(_0, Int::from(1_u64)), bt(_0, Int::from(1_u64)));
            assert_eq!(bt(_0, Int::from(2_u64)) - bt(_0, Int::from(0_u64)), bt(_0, Int::from(2_u64)));
            assert_eq!(bt(_0, Int::from(2_u64)) - bt(_0, Int::from(2_u64)), bt(_0, Int::from(0_u64)));

            // works for refs
            let a = Int::from(10_u64);
            let b = Int::from(3_u64);
            let expected = Int::from(7_u64);
            dts!(_0, a, b, expected);
            assert_eq!(a - b, expected);
        }
    );

    int_test!( sub_overflow_panics,
        NoArgs
        attrs = #[should_panic(expected = "subtraction overflow")]
        => |_0| {
            let _ = bt(_0, Int::from(1_u64)) - bt(_0, Int::from(2_u64));
        }
    );

    int_test!( sub_assign_works,
        NoArgs
        attrs = #[allow(clippy::op_ref)]
        => |_0| {
            let mut a = bt(_0, Int::from(14_u64));
            a -= bt(_0, Int::from(2_u64));
            assert_eq!(a, bt(_0, Int::from(12_u64)));
        }
    );

    int_test!( mul,
        NoArgs
        attrs = #[allow(clippy::op_ref)]
        => |_0| {
            assert_eq!(bt(_0, Int::from(2_u32)) * bt(_0, Int::from(3_u32)), bt(_0, Int::from(6_u32)));
            assert_eq!(bt(_0, Int::from(2_u32)) * bt(_0, Int::ZERO), bt(_0, Int::ZERO));

            // works for refs
            let a = bt(_0, Int::from(11_u32));
            let b = bt(_0, Int::from(3_u32));
            let expected = bt(_0, Int::from(33_u32));
            assert_eq!(a * b, expected);
        }
    );

    int_test!( mul_overflow_panics,
        NoArgs
        attrs = #[should_panic(expected = "multiplication overflow")]
        => |_0| {
            let max = bt(_0, Int::MAX);
            let _ = max * bt(_0, Int::from(2_u64));
        }
    );

    int_test!( mul_assign_works,
        NoArgs
        attrs = #[allow(clippy::op_ref)]
        => |_0| {
            let mut a = bt(_0, Int::from(14_u32));
            a *= bt(_0, Int::from(2_u32));
            assert_eq!(a, bt(_0, Int::from(28_u32)));
        }
    );

    int_test! (pow_works,
        NoArgs
        => |_0| {
            assert_eq!(bt(_0, Int::from(2_u32)).checked_pow(2).unwrap(), bt(_0, Int::from(4_u32)));
            assert_eq!(bt(_0, Int::from(2_u32)).checked_pow(10).unwrap(), bt(_0, Int::from(1024_u32)));

            // overflow
            let max = bt(_0, Int::MAX);
            let result = max.checked_pow(2);
            let MathError::OverflowPow { .. } = result.unwrap_err() else {
                panic!("Expected OverflowPow error");
            };

        }
    );

    int_test!( multiply_ratio,
        NoArgs
        => |_0| {
            let base = Int::from(500_u64);
            let min = Int::MIN;
            let max = Int::MAX;
            dts!(_0, base, min, max);

            // factor 1/1
            assert_eq!(base.checked_multiply_ratio_ceil(1_u64, 1_u64).unwrap(), base);
            assert_eq!(base.checked_multiply_ratio_ceil(3_u64, 3_u64).unwrap(), base);
            assert_eq!(base.checked_multiply_ratio_ceil(654321_u64, 654321_u64).unwrap(), base);
            assert_eq!(base.checked_multiply_ratio_ceil(max, max).unwrap(), base);

            assert_eq!(base.checked_multiply_ratio_floor(1_u64, 1_u64).unwrap(), base);
            assert_eq!(base.checked_multiply_ratio_floor(3_u64, 3_u64).unwrap(), base);
            assert_eq!(base.checked_multiply_ratio_floor(654321_u64, 654321_u64).unwrap(), base);
            assert_eq!(base.checked_multiply_ratio_floor(max, max).unwrap(), base);

            // factor 3/2
            assert_eq!(base.checked_multiply_ratio_ceil(3_u64, 2_u64).unwrap(), Int::from(750_u64));
            assert_eq!(base.checked_multiply_ratio_floor(3_u64, 2_u64).unwrap(), Int::from(750_u64));
            assert_eq!(base.checked_multiply_ratio_ceil(333333_u64, 222222_u64).unwrap(), Int::from(750_u64));
            assert_eq!(base.checked_multiply_ratio_floor(333333_u64, 222222_u64).unwrap(), Int::from(750_u64));

            // factor 2/3
            assert_eq!(base.checked_multiply_ratio_ceil(2_u64, 3_u64).unwrap(), Int::from(334_u64));
            assert_eq!(base.checked_multiply_ratio_floor(2_u64, 3_u64).unwrap(), Int::from(333_u64));
            assert_eq!(base.checked_multiply_ratio_ceil(222222_u64, 333333_u64).unwrap(), Int::from(334_u64));
            assert_eq!(base.checked_multiply_ratio_floor(222222_u64, 333333_u64).unwrap(), Int::from(333_u64));

            // factor 5/6
            assert_eq!(base.checked_multiply_ratio_ceil(5_u64, 6_u64).unwrap(), Int::from(417_u64));
            assert_eq!(base.checked_multiply_ratio_floor(5_u64, 6_u64).unwrap(), Int::from(416_u64));
            assert_eq!(base.checked_multiply_ratio_ceil(100_u64, 120_u64).unwrap(), Int::from(417_u64));
            assert_eq!(base.checked_multiply_ratio_floor(100_u64, 120_u64).unwrap(), Int::from(416_u64));


            // 0 num works
            assert_eq!(base.checked_multiply_ratio_ceil(_0, 1_u64).unwrap(), _0);
            assert_eq!(base.checked_multiply_ratio_floor(_0, 1_u64).unwrap(), _0);

            // 1 num works
            assert_eq!(base.checked_multiply_ratio_ceil(1_u64, 1_u64).unwrap(), base);
            assert_eq!(base.checked_multiply_ratio_floor(1_u64, 1_u64).unwrap(), base);

            // not round on even divide
            let _2 = bt(_0, Int::from(2_u64));

            assert_eq!(_2.checked_multiply_ratio_ceil(6_u64, 4_u64).unwrap(), Int::from(3_u64));
            assert_eq!(_2.checked_multiply_ratio_floor(6_u64, 4_u64).unwrap(), Int::from(3_u64));

        }
    );

    int_test!( multiply_ratio_does_not_overflow_when_result_fits,
        NoArgs
         => |_0| {
            // Almost max value for Uint128.
            let max = Int::MAX;
            let reduce = Int::from(9_u64);
            let base = max - reduce;
            dts!(_0, base, max, reduce);

            assert_eq!(base.checked_multiply_ratio_ceil(2_u64, 2_u64).unwrap(), base);
            assert_eq!(base.checked_multiply_ratio_floor(2_u64, 2_u64).unwrap(), base);
        }
    );

    int_test!( multiply_ratio_overflow,
        NoArgs
        => |_0| {
            // Almost max value for Uint128.
            let max = Int::MAX;
            let reduce = Int::from(9_u64);
            let base = max - reduce;
            dts!(_0, base, max, reduce);

            let result = base.checked_multiply_ratio_ceil(2_u64, 1_u64);
            let MathError::OverflowConversion { .. } = result.unwrap_err() else {
                panic!("Expected OverflowConversion error");
            };

            let result = base.checked_multiply_ratio_floor(2_u64, 1_u64);
            let MathError::OverflowConversion { .. } = result.unwrap_err() else {
                panic!("Expected OverflowConversion error");
            };
        }
    );

    int_test!( multiply_ratio_divide_by_zero,
        NoArgs
        => |_0| {
            let base = bt(_0, Int::from(500_u64));

            let result = base.checked_multiply_ratio_ceil(1_u64, 0_u64);
            let MathError::DivisionByZero { .. } = result.unwrap_err() else {
                panic!("Expected DivisionByZero error");
            };

            let result = base.checked_multiply_ratio_floor(1_u64, 0_u64);
            let MathError::DivisionByZero { .. } = result.unwrap_err() else {
                panic!("Expected DivisionByZero error");
            };
        }
    );

    int_test! (shr,
        NoArgs
        => |_0| {
            let original = bt(_0, Int::from(160_u64));
            assert_eq!(original >> 1, bt(_0, Int::from(80_u64)));
            assert_eq!(original >> 3, bt(_0, Int::from(20_u64)));
            assert_eq!(original >> 2, bt(_0, Int::from(40_u64)));
        }
    );

    int_test!( shr_overflow_panics,
        Specific
        u128 = [128]
        u256 = [256]
        attrs = #[should_panic(expected = "shift overflow")]
        => |u, shift| {
            let original = bt(u, Int::from(1_u64));
            let _ = original >> shift;
        }
    );

    int_test! (shl,
        NoArgs
        => |_0| {
            let original = bt(_0, Int::from(160_u64));
            assert_eq!(original << 1, bt(_0, Int::from(320_u64)));
            assert_eq!(original << 2, bt(_0, Int::from(640_u64)));
            assert_eq!(original << 3, bt(_0, Int::from(1280_u64)));
        }
    );

    int_test!( shl_overflow_panics,
        Specific
        u128 = [128]
        u256 = [256]
        attrs = #[should_panic(expected = "shift overflow")]
        => |_0, shift| {
            let original = bt(_0, Int::from(1_u64));
            let _ = original << shift;
        }
    );

    int_test!( methods,
        NoArgs
        => |_0| {
            let max = Int::MAX;
            let _1 = Int::ONE;
            let _2 = Int::from(2_u64);
            dts!(_0, max, _1, _2);


            // checked_*
            assert!(matches!(
                max.checked_add(_1),
                Err(MathError::OverflowAdd { .. })
            ));

            assert_eq!(_1.checked_add(Int::from(1_u64)).unwrap(), Int::from(2_u64));
            assert!(matches!(
                _0.checked_sub(_1),
                Err(MathError::OverflowSub { .. })
            ));

            assert_eq!(Int::from(2_u64).checked_sub(_1).unwrap(), _1);

            assert!(matches!(
                max.checked_mul(Int::from(2_u64)),
                Err(MathError::OverflowMul { .. })
            ));

            assert_eq!(_2.checked_mul(_2).unwrap(), Int::from(4_u64));

            assert!(matches!(
                max.checked_pow(2u32),
                Err(MathError::OverflowPow { .. })
            ));

            assert_eq!(_2.checked_pow(3).unwrap(), Int::from(8_u64));

            assert!(matches!(
                max.checked_div(_0),
                Err(MathError::DivisionByZero { .. })
            ));

            assert_eq!(Int::from(6_u64).checked_div(_2).unwrap(), Int::from(3_u64));

            assert!(matches!(
                max.checked_rem(_0),
                Err(MathError::DivisionByZero { .. })
            ));

            // saturating_*
            assert_eq!(max.saturating_add(Int::from(1_u64)), max);
            assert_eq!(_0.saturating_sub(Int::from(1_u64)), _0);
            assert_eq!(max.saturating_mul(Int::from(2_u64)), max);
            assert_eq!(max.saturating_pow(2), max);
        }
    );

    int_test!( wrapping_methods,
        NoArgs
        => |_0| {
            let max = Int::MAX;
            let _1 = Int::ONE;
            let _2 = Int::from(2_u64);
            dts!(_0, _1, _2, max);

            // wrapping_add
            assert_eq!(_2.wrapping_add(_2), Int::from(4_u64)); // non-wrapping
            assert_eq!(max.wrapping_add(_1), _0); // wrapping

            // wrapping_sub
            assert_eq!(Int::from(7_u64).wrapping_sub(Int::from(5_u64)), _2); // non-wrapping
            assert_eq!(_0.wrapping_sub(_1), max); // wrapping

            // wrapping_mul
            assert_eq!(_2.wrapping_mul(_2), Int::from(4_u64)); // non-wrapping
            assert_eq!( max.wrapping_mul(_2), max - _1); // wrapping

            // wrapping_pow
            assert_eq!(_2.wrapping_pow(3), Int::from(8_u64)); // non-wrapping
            assert_eq!(max.wrapping_pow(2), Int::from(1_u64)); // wrapping
        }
    );

    int_test!( saturating_methods,
        NoArgs
        => |_0| {
            let max = Int::MAX;
            let _1 = Int::ONE;
            let _2 = Int::from(2_u64);
            dts!(_0, _1, _2, max);

            // saturating_add
            assert_eq!(_2.saturating_add(_2), Int::from(4_u64)); // non-saturating
            assert_eq!(max.saturating_add(_1), max); // saturating

            // saturating_sub
            assert_eq!(Int::from(7_u64).saturating_sub(Int::from(5_u64)), _2); // non-saturating
            assert_eq!(_0.saturating_sub(_1), _0); // saturating

            // saturating_mul
            assert_eq!(_2.saturating_mul(_2), Int::from(4_u64)); // non-saturating
            assert_eq!(max.saturating_mul(_2), max); // saturating

            // saturating_pow
            assert_eq!(_2.saturating_pow(3), Int::from(8_u64)); // non-saturating
            assert_eq!(max.saturating_pow(2), max); // saturating
        }
    );

    int_test!( rem,
        NoArgs
        attrs = #[allow(clippy::op_ref)]
        => |_0| {
            let _1 = Int::from(1_u64);
            let _10 = Int::from(10_u64);
            let _3 = Int::from(3_u64);
            dts!(_0, _1, _3, _3);

            assert_eq!(_10 % Int::from(10_u64), _0);
            assert_eq!(_10 % Int::from(2_u64), _0);
            assert_eq!(_10 % Int::from(1_u64), _0);
            assert_eq!(_10 % Int::from(3_u64), Int::from(1_u64));
            assert_eq!(_10 % Int::from(4_u64), Int::from(2_u64));
            assert_eq!(_10 % _3, _1);

            // works for assign
            let mut _30 = bt(_0, Int::from(30_u64));
            _30 %=  Int::from(4_u64);
            assert_eq!(_30, Int::from(2_u64));
        }
    );

    int_test!( rem_panics_for_zero,
        NoArgs
        attrs = #[should_panic(expected = "division by zero")]
        => |_0| {
            let _ = Int::from(10_u64) % _0;
        }
    );

    int_test!( partial_eq,
        NoArgs
        attrs = #[allow(clippy::op_ref)]
        => |_0| {
            let test_cases = [
                    (1_u64, 1_u64, true),
                    (42_u64, 42_u64, true),
                    (42_u64, 24_u64, false),
                    (0_u64, 0_u64, true)
                ]
                .into_iter()
                .map(|(lhs, rhs, expected)|
                    (
                        bt(_0, Int::from(lhs)),
                        bt(_0, Int::from(rhs)),
                        expected
                    )
                );

            for (lhs, rhs, expected) in test_cases {
                assert_eq!(lhs == rhs, expected);
            }
        }
    );

    int_test!( mul_floor,
        Specific
        u128 = [Udec128::new(2_u128), Udec128::from_str("0.5").unwrap(), Udec128::from_str("1.5").unwrap()]
        u256 = [Udec256::new(2_u128), Udec256::from_str("0.5").unwrap(), Udec256::from_str("1.5").unwrap()]
        => |_0, _2d, _0_5d, _1_5d| {
            let _1 = Int::from(1_u64);
            let _2 = Int::from(2_u64);
            let _10 = Int::from(10_u64);
            let _11 = Int::from(11_u64);
            let max = Int::MAX;
            dts!(_0, _1, _2, _10, _11, max);

            assert_eq!(_10.checked_mul_dec_ceil(_2d).unwrap(), Int::from(20_u64));
            assert_eq!(_10.checked_mul_dec_floor(_2d).unwrap(), Int::from(20_u64));

            assert_eq!(_10.checked_mul_dec_ceil(_1_5d).unwrap(), Int::from(15_u64));
            assert_eq!(_10.checked_mul_dec_floor(_1_5d).unwrap(), Int::from(15_u64));

            assert_eq!(_10.checked_mul_dec_ceil(_0_5d).unwrap(), Int::from(5_u64));
            assert_eq!(_10.checked_mul_dec_floor(_0_5d).unwrap(), Int::from(5_u64));

            // ceil works
            assert_eq!(_11.checked_mul_dec_floor(_0_5d).unwrap(), Int::from(5_u64));
            assert_eq!(_11.checked_mul_dec_ceil(_0_5d).unwrap(), Int::from(6_u64));

            // overflow num but not overflow result
            assert_eq!(max.checked_mul_dec_ceil(_0_5d).unwrap(), max / _2 + _1);
            assert_eq!(max.checked_mul_dec_floor(_0_5d).unwrap(), max / _2);

            // overflow num and overflow result
            assert!(matches!(
                max.checked_mul_dec_ceil(_2d),
                Err(MathError::OverflowConversion { .. })
            ));
            assert!(matches!(
                max.checked_mul_dec_floor(_2d),
                Err(MathError::OverflowConversion { .. })
            ));
        }
    );

    int_test!( div_floor,
        Specific
        u128 = [Udec128::new(0_u128), Udec128::new(2_u128), Udec128::from_str("0.5").unwrap(), Udec128::from_str("1.5").unwrap()]
        u256 = [Udec256::new(0_u128), Udec256::new(2_u128), Udec256::from_str("0.5").unwrap(), Udec256::from_str("1.5").unwrap()]
        => |_0, _0d, _2d, _0_5d, _1_5d| {
            let _1 = Int::from(1_u64);
            let _2 = Int::from(2_u64);
            let _10 = Int::from(10_u64);
            let _11 = Int::from(11_u64);
            let max = Int::MAX;
            dts!(_0, _1, _2, _10, _11,  max);

            assert_eq!(_11.checked_div_dec_floor(_2d).unwrap(), Int::from(5_u64));
            assert_eq!(_11.checked_div_dec_ceil(_2d).unwrap(), Int::from(6_u64));

            assert_eq!(_10.checked_div_dec_floor(_2d).unwrap(), Int::from(5_u64));
            assert_eq!(_10.checked_div_dec_ceil(_2d).unwrap(), Int::from(5_u64));

            // ceil works
            assert_eq!(_11.checked_div_dec_floor(_1_5d).unwrap(), Int::from(7_u64));
            assert_eq!(_11.checked_div_dec_ceil(_1_5d).unwrap(), Int::from(8_u64));

            // overflow num but not overflow result
            assert_eq!(max.checked_div_dec_floor(_2d).unwrap(), max / _2);
            assert_eq!(max.checked_div_dec_ceil(_2d).unwrap(), max / _2 + _1);

            // overflow num and overflow result
            assert!(matches!(
                max.checked_div_dec_floor(_0_5d),
                Err(MathError::OverflowConversion { .. })
            ));
            assert!(matches!(
                max.checked_div_dec_ceil(_0_5d),
                Err(MathError::OverflowConversion { .. })
            ));

            // Divide by zero
            assert!(matches!(
                _10.checked_div_dec_floor(_0d),
                Err(MathError::DivisionByZero { .. })
            ));
            assert!(matches!(
                _10.checked_div_dec_ceil(_0d),
                Err(MathError::DivisionByZero { .. })
            ));
        }
    );
}
