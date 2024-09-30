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

#[cfg(test)]
pub mod testse {

    use {
        super::*,
        crate::{
            dts, int_test,
            test_utils::{bt, dt},
            NumberConst,
        },
    };

    int_test!( size_of,
        Specific
        u128 = [16]
        u256 = [32]
        i128 = [16]
        i256 = [32]
        => |_0, size| {
            assert_eq!(core::mem::size_of_val(&_0), size);
        }
    );

    int_test!( from_str,
        Specific
        u128 = [[(128_u128, "128")]]
        u256 = [[(U256::from(256_u128), "256")]]
        i128 = [[(-128_i128, "-128"), (-128_i128, "-128")]]
        i256 = [[(I256::from(256_i128), "256"), (I256::from(-256_i128), "-256")]]
        => |_, samples| {
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

    int_test!( display,
        Specific
        u128 = [[(Uint128::new(128_u128), "128")]]
        u256 = [[(Uint256::new(U256::from(256_u128)), "256")]]
        i128 = [[
                    (Int128::MAX, "170141183460469231731687303715884105727"),
                    (Int128::MIN, "-170141183460469231731687303715884105728"),
                    (Int128::new(i128::ZERO), "0"),
                ]]
        i256 = [[
            (Int256::MAX, "57896044618658097711785492504343953926634992332820282019728792003956564819967"),
            (Int256::MIN, "-57896044618658097711785492504343953926634992332820282019728792003956564819968"),
            (Int256::ZERO, "0"),

        ]]
        => |_, samples| {
            for (number, str) in samples {
                assert_eq!(format!("{}", number), str);
            }
        }
    );

    int_test!( display_padding_front,
        Specific
        u128 = [[("00128", "128")]]
        u256 = [[("000256", "256")]]
        i128 = [[
                    ("000128", "128"),
                    ("-000128", "-128"),
                ]]
        i256 = [[
                    ("000256", "256"),
                    ("-000256", "-256"),
                ]]
        => |_0, samples| {
            for (padded_str, compare) in samples {
                let uint = bt(_0, Int::from_str(padded_str).unwrap());
                assert_eq!(format!("{}", uint), compare);
            }
        }
    );

    int_test!( json,
        Specific
        u128 = [["123456"]]
        u256 = [["123456"]]
        i128 = [["123456", "-123456"]]
        i256 = [["123456", "-123456"]]

    => |_0, samples| {

        for sample in samples {
            let original = bt(_0, Int::from_str(sample).unwrap());

            let serialized_str = serde_json::to_string(&original).unwrap();
            assert_eq!(serialized_str, format!("\"{}\"", sample));

            let serialized_vec = serde_json::to_vec(&original).unwrap();
            assert_eq!(serialized_vec, format!("\"{}\"", sample).as_bytes());

            let parsed: Int::<_> = serde_json::from_str(&serialized_str).unwrap();
            assert_eq!(parsed, original);

            let parsed: Int::<_> = serde_json::from_slice(&serialized_vec).unwrap();
            assert_eq!(parsed, original);
        }
    });

    int_test!( compare,
        Specific
        u128 = [[(10_u128, 200_u128)]]
        u256 = [[(U256::from(10_u128), U256::from(200_u128))]]
        i128 = [[
                    (10_i128, 200_i128),
                    (-10, 200),
                    (-200, -10)
                ]]
        i256 = [[
                    (I256::from(10), I256::from(200_i128)),
                    (I256::from(-10), I256::from(200)),
                    (I256::from(-200), I256::from(-10))
                 ]]
        => |_0, samples| {
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

    // int_test!( multiply_ratio,
    //     Specific
    //     u128 = []
    //     u256 = []
    //     i128 = []
    //     // TODO: Missing i256 casue From<I256> for I512 is not implemented yet
    //     => |_0| {
    //         let base = Int::from(500_u64);
    //         let min = Int::MIN;
    //         let max = Int::MAX;
    //         dts!(_0, base, min, max);

    //         // factor 1/1
    //         assert_eq!(base.checked_multiply_ratio_ceil(1_u64, 1_u64).unwrap(), base);
    //         assert_eq!(base.checked_multiply_ratio_ceil(3_u64, 3_u64).unwrap(), base);
    //         assert_eq!(base.checked_multiply_ratio_ceil(654321_u64, 654321_u64).unwrap(), base);
    //         assert_eq!(base.checked_multiply_ratio_ceil(max, max).unwrap(), base);

    //         assert_eq!(base.checked_multiply_ratio_floor(1_u64, 1_u64).unwrap(), base);
    //         assert_eq!(base.checked_multiply_ratio_floor(3_u64, 3_u64).unwrap(), base);
    //         assert_eq!(base.checked_multiply_ratio_floor(654321_u64, 654321_u64).unwrap(), base);
    //         assert_eq!(base.checked_multiply_ratio_floor(max, max).unwrap(), base);

    //         // factor 3/2
    //         assert_eq!(base.checked_multiply_ratio_ceil(3_u64, 2_u64).unwrap(), Int::from(750_u64));
    //         assert_eq!(base.checked_multiply_ratio_floor(3_u64, 2_u64).unwrap(), Int::from(750_u64));
    //         assert_eq!(base.checked_multiply_ratio_ceil(333333_u64, 222222_u64).unwrap(), Int::from(750_u64));
    //         assert_eq!(base.checked_multiply_ratio_floor(333333_u64, 222222_u64).unwrap(), Int::from(750_u64));

    //         // factor 2/3
    //         assert_eq!(base.checked_multiply_ratio_ceil(2_u64, 3_u64).unwrap(), Int::from(334_u64));
    //         assert_eq!(base.checked_multiply_ratio_floor(2_u64, 3_u64).unwrap(), Int::from(333_u64));
    //         assert_eq!(base.checked_multiply_ratio_ceil(222222_u64, 333333_u64).unwrap(), Int::from(334_u64));
    //         assert_eq!(base.checked_multiply_ratio_floor(222222_u64, 333333_u64).unwrap(), Int::from(333_u64));

    //         // factor 5/6
    //         assert_eq!(base.checked_multiply_ratio_ceil(5_u64, 6_u64).unwrap(), Int::from(417_u64));
    //         assert_eq!(base.checked_multiply_ratio_floor(5_u64, 6_u64).unwrap(), Int::from(416_u64));
    //         assert_eq!(base.checked_multiply_ratio_ceil(100_u64, 120_u64).unwrap(), Int::from(417_u64));
    //         assert_eq!(base.checked_multiply_ratio_floor(100_u64, 120_u64).unwrap(), Int::from(416_u64));

    //         // 0 num works
    //         assert_eq!(base.checked_multiply_ratio_ceil(_0, 1_u64).unwrap(), _0);
    //         assert_eq!(base.checked_multiply_ratio_floor(_0, 1_u64).unwrap(), _0);

    //         // 1 num works
    //         assert_eq!(base.checked_multiply_ratio_ceil(1_u64, 1_u64).unwrap(), base);
    //         assert_eq!(base.checked_multiply_ratio_floor(1_u64, 1_u64).unwrap(), base);

    //         // not round on even divide
    //         let _2 = bt(_0, Int::from(2_u64));

    //         assert_eq!(_2.checked_multiply_ratio_ceil(6_u64, 4_u64).unwrap(), Int::from(3_u64));
    //         assert_eq!(_2.checked_multiply_ratio_floor(6_u64, 4_u64).unwrap(), Int::from(3_u64));

    //     }
    // );

    // int_test!( multiply_ratio_does_not_overflow_when_result_fits,
    //     Specific
    //     u128 = []
    //     u256 = []
    //     i128 = []
    //     // TODO: Missing i256 casue From<I256> for I512 is not implemented yet
    //      => |_0| {
    //         // Almost max value for Uint128.
    //         let max = Int::MAX;
    //         let reduce = Int::from(9_u64);
    //         let base = max - reduce;
    //         dts!(_0, base, max, reduce);

    //         assert_eq!(base.checked_multiply_ratio_ceil(2_u64, 2_u64).unwrap(), base);
    //         assert_eq!(base.checked_multiply_ratio_floor(2_u64, 2_u64).unwrap(), base);
    //     }
    // );

    // int_test!( multiply_ratio_overflow,
    //     Specific
    //     u128 = []
    //     u256 = []
    //     i128 = []
    //     // TODO: Missing i256 casue From<I256> for I512 is not implemented yet
    //     => |_0| {
    //         // Almost max value for Uint128.
    //         let max = Int::MAX;
    //         let reduce = Int::from(9_u64);
    //         let base = max - reduce;
    //         dts!(_0, base, max, reduce);

    //         let result = base.checked_multiply_ratio_ceil(2_u64, 1_u64);
    //         let MathError::OverflowConversion { .. } = result.unwrap_err() else {
    //             panic!("Expected OverflowConversion error");
    //         };

    //         let result = base.checked_multiply_ratio_floor(2_u64, 1_u64);
    //         let MathError::OverflowConversion { .. } = result.unwrap_err() else {
    //             panic!("Expected OverflowConversion error");
    //         };
    //     }
    // );

    // int_test!( multiply_ratio_divide_by_zero,
    //     Specific
    //     u128 = []
    //     u256 = []
    //     i128 = []
    //     // TODO: Missing i256 casue From<I256> for I512 is not implemented yet
    //     => |_0| {
    //         let base = bt(_0, Int::from(500_u64));

    //         let result = base.checked_multiply_ratio_ceil(1_u64, 0_u64);
    //         let MathError::DivisionByZero { .. } = result.unwrap_err() else {
    //             panic!("Expected DivisionByZero error");
    //         };

    //         let result = base.checked_multiply_ratio_floor(1_u64, 0_u64);
    //         let MathError::DivisionByZero { .. } = result.unwrap_err() else {
    //             panic!("Expected DivisionByZero error");
    //         };
    //     }
    // );

    // int_test! (shr,
    //     NoArgs
    //     => |_0| {
    //         let original = bt(_0, Int::from(160_u64));
    //         assert_eq!(original >> 1, bt(_0, Int::from(80_u64)));
    //         assert_eq!(original >> 3, bt(_0, Int::from(20_u64)));
    //         assert_eq!(original >> 2, bt(_0, Int::from(40_u64)));
    //     }
    // );

    // int_test!( shr_overflow_panics,
    //     Specific
    //     u128 = [128]
    //     u256 = [256]
    //     i128 = [128]
    //     i256 = [256]
    //     attrs = #[should_panic(expected = "shift overflow")]
    //     => |u, shift| {
    //         let original = bt(u, Int::from(1_u64));
    //         let _ = original >> shift;
    //     }
    // );

    // int_test! (shl,
    //     NoArgs
    //     => |_0| {
    //         let original = bt(_0, Int::from(160_u64));
    //         assert_eq!(original << 1, bt(_0, Int::from(320_u64)));
    //         assert_eq!(original << 2, bt(_0, Int::from(640_u64)));
    //         assert_eq!(original << 3, bt(_0, Int::from(1280_u64)));
    //     }
    // );

    // int_test!( shl_overflow_panics,
    //     Specific
    //     u128 = [128]
    //     u256 = [256]
    //     i128 = [128]
    //     i256 = [256]
    //     attrs = #[should_panic(expected = "shift overflow")]
    //     => |_0, shift| {
    //         let original = bt(_0, Int::from(1_u64));
    //         let _ = original << shift;
    //     }
    // );

    // int_test!( rem,
    //     NoArgs
    //     attrs = #[allow(clippy::op_ref)]
    //     => |_0| {
    //         let _1 = Int::from(1_u64);
    //         let _10 = Int::from(10_u64);
    //         let _3 = Int::from(3_u64);
    //         dts!(_0, _1, _3, _3);

    //         assert_eq!(_10 % Int::from(10_u64), _0);
    //         assert_eq!(_10 % Int::from(2_u64), _0);
    //         assert_eq!(_10 % Int::from(1_u64), _0);
    //         assert_eq!(_10 % Int::from(3_u64), Int::from(1_u64));
    //         assert_eq!(_10 % Int::from(4_u64), Int::from(2_u64));
    //         assert_eq!(_10 % _3, _1);

    //         // works for assign
    //         let mut _30 = bt(_0, Int::from(30_u64));
    //         _30 %=  Int::from(4_u64);
    //         assert_eq!(_30, Int::from(2_u64));
    //     }
    // );

    // int_test!( rem_panics_for_zero,
    //     NoArgs
    //     attrs = #[should_panic(expected = "division by zero")]
    //     => |_0| {
    //         let _ = Int::from(10_u64) % _0;
    //     }
    // );

    // int_test!( partial_eq,
    //     NoArgs
    //     attrs = #[allow(clippy::op_ref)]
    //     => |_0| {
    //         let test_cases = [
    //                 (1_u64, 1_u64, true),
    //                 (42_u64, 42_u64, true),
    //                 (42_u64, 24_u64, false),
    //                 (0_u64, 0_u64, true)
    //             ]
    //             .into_iter()
    //             .map(|(lhs, rhs, expected)|
    //                 (
    //                     bt(_0, Int::from(lhs)),
    //                     bt(_0, Int::from(rhs)),
    //                     expected
    //                 )
    //             );

    //         for (lhs, rhs, expected) in test_cases {
    //             assert_eq!(lhs == rhs, expected);
    //         }
    //     }
    // );

    // int_test!( mul_floor,
    //     Specific
    //     u128 = [Udec128::new(2_u128), Udec128::from_str("0.5").unwrap(), Udec128::from_str("1.5").unwrap()]
    //     u256 = [Udec256::new(2_u128), Udec256::from_str("0.5").unwrap(), Udec256::from_str("1.5").unwrap()]
    //     => |_0, _2d, _0_5d, _1_5d| {
    //         let _1 = Int::from(1_u64);
    //         let _2 = Int::from(2_u64);
    //         let _10 = Int::from(10_u64);
    //         let _11 = Int::from(11_u64);
    //         let max = Int::MAX;
    //         dts!(_0, _1, _2, _10, _11, max);

    //         assert_eq!(_10.checked_mul_dec_ceil(_2d).unwrap(), Int::from(20_u64));
    //         assert_eq!(_10.checked_mul_dec_floor(_2d).unwrap(), Int::from(20_u64));

    //         assert_eq!(_10.checked_mul_dec_ceil(_1_5d).unwrap(), Int::from(15_u64));
    //         assert_eq!(_10.checked_mul_dec_floor(_1_5d).unwrap(), Int::from(15_u64));

    //         assert_eq!(_10.checked_mul_dec_ceil(_0_5d).unwrap(), Int::from(5_u64));
    //         assert_eq!(_10.checked_mul_dec_floor(_0_5d).unwrap(), Int::from(5_u64));

    //         // ceil works
    //         assert_eq!(_11.checked_mul_dec_floor(_0_5d).unwrap(), Int::from(5_u64));
    //         assert_eq!(_11.checked_mul_dec_ceil(_0_5d).unwrap(), Int::from(6_u64));

    //         // overflow num but not overflow result
    //         assert_eq!(max.checked_mul_dec_ceil(_0_5d).unwrap(), max / _2 + _1);
    //         assert_eq!(max.checked_mul_dec_floor(_0_5d).unwrap(), max / _2);

    //         // overflow num and overflow result
    //         assert!(matches!(
    //             max.checked_mul_dec_ceil(_2d),
    //             Err(MathError::OverflowConversion { .. })
    //         ));
    //         assert!(matches!(
    //             max.checked_mul_dec_floor(_2d),
    //             Err(MathError::OverflowConversion { .. })
    //         ));
    //     }
    // );

    // int_test!( div_floor,
    //     Specific
    //     u128 = [Udec128::new(0_u128), Udec128::new(2_u128), Udec128::from_str("0.5").unwrap(), Udec128::from_str("1.5").unwrap()]
    //     u256 = [Udec256::new(0_u128), Udec256::new(2_u128), Udec256::from_str("0.5").unwrap(), Udec256::from_str("1.5").unwrap()]
    //     => |_0, _0d, _2d, _0_5d, _1_5d| {
    //         let _1 = Int::from(1_u64);
    //         let _2 = Int::from(2_u64);
    //         let _10 = Int::from(10_u64);
    //         let _11 = Int::from(11_u64);
    //         let max = Int::MAX;
    //         dts!(_0, _1, _2, _10, _11,  max);

    //         assert_eq!(_11.checked_div_dec_floor(_2d).unwrap(), Int::from(5_u64));
    //         assert_eq!(_11.checked_div_dec_ceil(_2d).unwrap(), Int::from(6_u64));

    //         assert_eq!(_10.checked_div_dec_floor(_2d).unwrap(), Int::from(5_u64));
    //         assert_eq!(_10.checked_div_dec_ceil(_2d).unwrap(), Int::from(5_u64));

    //         // ceil works
    //         assert_eq!(_11.checked_div_dec_floor(_1_5d).unwrap(), Int::from(7_u64));
    //         assert_eq!(_11.checked_div_dec_ceil(_1_5d).unwrap(), Int::from(8_u64));

    //         // overflow num but not overflow result
    //         assert_eq!(max.checked_div_dec_floor(_2d).unwrap(), max / _2);
    //         assert_eq!(max.checked_div_dec_ceil(_2d).unwrap(), max / _2 + _1);

    //         // overflow num and overflow result
    //         assert!(matches!(
    //             max.checked_div_dec_floor(_0_5d),
    //             Err(MathError::OverflowConversion { .. })
    //         ));
    //         assert!(matches!(
    //             max.checked_div_dec_ceil(_0_5d),
    //             Err(MathError::OverflowConversion { .. })
    //         ));

    //         // Divide by zero
    //         assert!(matches!(
    //             _10.checked_div_dec_floor(_0d),
    //             Err(MathError::DivisionByZero { .. })
    //         ));
    //         assert!(matches!(
    //             _10.checked_div_dec_ceil(_0d),
    //             Err(MathError::DivisionByZero { .. })
    //         ));
    //     }
    // );
}
