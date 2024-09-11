use {
    crate::{
        forward_ref_binop_typed, forward_ref_op_assign_typed, generate_uint,
        impl_all_ops_and_assign, impl_assign_integer, impl_assign_number, impl_integer, impl_next,
        impl_number, Bytable, Fraction, Inner, Integer, MultiplyFraction, MultiplyRatio,
        NextNumber, Number, NumberConst, Sign, StdError, StdResult,
    },
    bnum::types::{U256, U512},
    borsh::{BorshDeserialize, BorshSerialize},
    forward_ref::{forward_ref_binop, forward_ref_op_assign},
    serde::{de, ser},
    std::{
        fmt::{self, Display},
        marker::PhantomData,
        ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign},
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

    // TODO: this necessary?
    pub fn new_from(value: impl Into<U>) -> Self {
        Self(value.into())
    }
}

impl<U> Uint<U>
where
    U: Copy,
{
    pub const fn number(&self) -> U {
        self.0
    }
}

impl<U> Inner for Uint<U> {
    type U = U;
}

impl<U> Sign for Uint<U> {
    fn abs(self) -> Self {
        self
    }

    fn is_negative(&self) -> bool {
        false
    }
}

impl<U> NumberConst for Uint<U>
where
    U: NumberConst,
{
    const MAX: Self = Self(U::MAX);
    const MIN: Self = Self(U::MIN);
    const ONE: Self = Self(U::ONE);
    const TEN: Self = Self(U::TEN);
    const ZERO: Self = Self(U::ZERO);
}

impl<U, const S: usize> Bytable<S> for Uint<U>
where
    U: Bytable<S>,
{
    fn from_be_bytes(data: [u8; S]) -> Self {
        Self(U::from_be_bytes(data))
    }

    fn from_le_bytes(data: [u8; S]) -> Self {
        Self(U::from_le_bytes(data))
    }

    fn to_be_bytes(self) -> [u8; S] {
        self.0.to_be_bytes()
    }

    fn to_le_bytes(self) -> [u8; S] {
        self.0.to_le_bytes()
    }

    fn grow_be_bytes<const INPUT_SIZE: usize>(data: [u8; INPUT_SIZE]) -> [u8; S] {
        U::grow_be_bytes(data)
    }

    fn grow_le_bytes<const INPUT_SIZE: usize>(data: [u8; INPUT_SIZE]) -> [u8; S] {
        U::grow_le_bytes(data)
    }
}

impl<U> Number for Uint<U>
where
    U: Number,
{
    fn is_zero(&self) -> bool {
        self.0.is_zero()
    }

    fn abs(self) -> Self {
        // `Uint` represents an unsigned integer, so the absolute value is
        // sipmly itself.
        self
    }

    fn checked_add(self, other: Self) -> StdResult<Self> {
        self.0.checked_add(other.0).map(Self)
    }

    fn checked_sub(self, other: Self) -> StdResult<Self> {
        self.0.checked_sub(other.0).map(Self)
    }

    fn checked_mul(self, other: Self) -> StdResult<Self> {
        self.0.checked_mul(other.0).map(Self)
    }

    fn checked_div(self, other: Self) -> StdResult<Self> {
        self.0.checked_div(other.0).map(Self)
    }

    fn checked_rem(self, other: Self) -> StdResult<Self> {
        self.0.checked_rem(other.0).map(Self)
    }

    fn checked_pow(self, other: u32) -> StdResult<Self> {
        self.0.checked_pow(other).map(Self)
    }

    fn checked_sqrt(self) -> StdResult<Self> {
        self.0.checked_sqrt().map(Self)
    }

    fn wrapping_add(self, other: Self) -> Self {
        Self(self.0.wrapping_add(other.0))
    }

    fn wrapping_sub(self, other: Self) -> Self {
        Self(self.0.wrapping_sub(other.0))
    }

    fn wrapping_mul(self, other: Self) -> Self {
        Self(self.0.wrapping_mul(other.0))
    }

    fn wrapping_pow(self, other: u32) -> Self {
        Self(self.0.wrapping_pow(other))
    }

    fn saturating_add(self, other: Self) -> Self {
        Self(self.0.saturating_add(other.0))
    }

    fn saturating_sub(self, other: Self) -> Self {
        Self(self.0.saturating_sub(other.0))
    }

    fn saturating_mul(self, other: Self) -> Self {
        Self(self.0.saturating_mul(other.0))
    }

    fn saturating_pow(self, other: u32) -> Self {
        Self(self.0.saturating_pow(other))
    }
}

impl<U> Integer for Uint<U>
where
    U: Integer,
{
    fn checked_ilog2(self) -> StdResult<u32> {
        self.0.checked_ilog2()
    }

    fn checked_ilog10(self) -> StdResult<u32> {
        self.0.checked_ilog10()
    }

    fn checked_shl(self, other: u32) -> StdResult<Self> {
        self.0.checked_shl(other).map(Self)
    }

    fn checked_shr(self, other: u32) -> StdResult<Self> {
        self.0.checked_shr(other).map(Self)
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
    ) -> StdResult<<Uint<U> as NextNumber>::Next> {
        let s = self.into_next();
        let r = rhs.into().into_next();
        s.checked_mul(r)
    }
}

impl<U> MultiplyRatio for Uint<U>
where
    Uint<U>: NextNumber + NumberConst + Number + Copy,
    <Uint<U> as NextNumber>::Next: Number + ToString + Clone,
{
    fn checked_multiply_ratio_floor<A: Into<Self>, B: Into<Self>>(
        self,
        numerator: A,
        denominator: B,
    ) -> StdResult<Self> {
        let denominator = denominator.into().into_next();
        let next_result = self.checked_full_mul(numerator)?.checked_div(denominator)?;
        next_result
            .clone()
            .try_into()
            .map_err(|_| StdError::overflow_conversion::<_, Self>(next_result))
    }

    fn checked_multiply_ratio_ceil<A: Into<Self>, B: Into<Self>>(
        self,
        numerator: A,
        denominator: B,
    ) -> StdResult<Self> {
        let numerator: Self = numerator.into();
        let dividend = self.checked_full_mul(numerator)?;
        let floor_result = self.checked_multiply_ratio_floor(numerator, denominator)?;
        let remained = dividend.checked_rem(floor_result.into_next())?;
        if !remained.is_zero() {
            floor_result.checked_add(Self::ONE)
        } else {
            Ok(floor_result)
        }
    }
}

impl<U, AsU, F> MultiplyFraction<F, AsU> for Uint<U>
where
    Uint<U>: NumberConst + Number + MultiplyRatio + From<Uint<AsU>> + ToString,
    F: Number + Fraction<AsU> + Sign + ToString,
{
    fn checked_mul_dec_floor(self, rhs: F) -> StdResult<Self> {
        // If either left or right hand side is zero, then simply return zero.
        if self.is_zero() || rhs.is_zero() {
            return Ok(Self::ZERO);
        }

        // The left hand side is `Uint`, a non-negative type, so multiplication
        // with any non-zero negative number goes out of bound.
        if rhs.is_negative() {
            return Err(StdError::negative_mul(self, rhs));
        }

        self.checked_multiply_ratio_floor(rhs.numerator(), F::denominator().into_inner())
    }

    fn checked_mul_dec_ceil(self, rhs: F) -> StdResult<Self> {
        if self.is_zero() || rhs.is_zero() {
            return Ok(Self::ZERO);
        }

        if rhs.is_negative() {
            return Err(StdError::negative_mul(self, rhs));
        }

        self.checked_multiply_ratio_ceil(rhs.numerator(), F::denominator().into_inner())
    }

    fn checked_div_dec_floor(self, rhs: F) -> StdResult<Self> {
        // If right hand side is zero, throw error, because you can't divide any
        // number by zero.
        if rhs.is_zero() {
            return Err(StdError::division_by_zero(self));
        }

        // If right hand side is negative, throw error, because you can't divide
        // and unsigned number with a negative number.
        if rhs.is_negative() {
            return Err(StdError::negative_div(self, rhs));
        }

        // If left hand side is zero, and we know right hand size is positive,
        // then simply return zero.
        if self.is_zero() {
            return Ok(Self::ZERO);
        }

        self.checked_multiply_ratio_floor(F::denominator().into_inner(), rhs.numerator())
    }

    fn checked_div_dec_ceil(self, rhs: F) -> StdResult<Self> {
        if rhs.is_zero() {
            return Err(StdError::division_by_zero(self));
        }

        if rhs.is_negative() {
            return Err(StdError::negative_div(self, rhs));
        }

        if self.is_zero() {
            return Ok(Self::ZERO);
        }

        self.checked_multiply_ratio_ceil(F::denominator().into_inner(), rhs.numerator())
    }
}

impl<U> FromStr for Uint<U>
where
    U: FromStr,
    <U as FromStr>::Err: ToString,
{
    type Err = StdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        U::from_str(s)
            .map(Self)
            .map_err(|err| StdError::parse_number::<Self, _, _>(s, err))
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

impl_number!(impl<U> Add, add for Uint<U> where sub fn checked_add);
impl_number!(impl<U> Sub, sub for Uint<U> where sub fn checked_sub);
impl_number!(impl<U> Mul, mul for Uint<U> where sub fn checked_mul);
impl_number!(impl<U> Div, div for Uint<U> where sub fn checked_div);
impl_integer!(impl<U> Shl, shl for Uint<U> where sub fn checked_shl, u32);
impl_integer!(impl<U> Shr, shr for Uint<U> where sub fn checked_shr, u32);

impl_assign_number!(impl<U> AddAssign, add_assign for Uint<U> where sub fn checked_add);
impl_assign_number!(impl<U> SubAssign, sub_assign for Uint<U> where sub fn checked_sub);
impl_assign_number!(impl<U> MulAssign, mul_assign for Uint<U> where sub fn checked_mul);
impl_assign_number!(impl<U> DivAssign, div_assign for Uint<U> where sub fn checked_div);
impl_assign_integer!(impl<U> ShrAssign, shr_assign for Uint<U> where sub fn checked_shr, u32);
impl_assign_integer!(impl<U> ShlAssign, shl_assign for Uint<U> where sub fn checked_shl, u32);

forward_ref_binop_typed!(impl<U> Add, add for Uint<U>, Uint<U>);
forward_ref_binop_typed!(impl<U> Sub, sub for Uint<U>, Uint<U>);
forward_ref_binop_typed!(impl<U> Mul, mul for Uint<U>, Uint<U>);
forward_ref_binop_typed!(impl<U> Div, div for Uint<U>, Uint<U>);
forward_ref_binop_typed!(impl<U> Rem, rem for Uint<U>, Uint<U>);
forward_ref_binop_typed!(impl<U> Shl, shl for Uint<U>, u32);
forward_ref_binop_typed!(impl<U> Shr, shr for Uint<U>, u32);

forward_ref_op_assign_typed!(impl<U> AddAssign, add_assign for Uint<U>, Uint<U>);
forward_ref_op_assign_typed!(impl<U> SubAssign, sub_assign for Uint<U>, Uint<U>);
forward_ref_op_assign_typed!(impl<U> MulAssign, mul_assign for Uint<U>, Uint<U>);
forward_ref_op_assign_typed!(impl<U> DivAssign, div_assign for Uint<U>, Uint<U>);
forward_ref_op_assign_typed!(impl<U> ShrAssign, shr_assign for Uint<U>, u32);
forward_ref_op_assign_typed!(impl<U> ShlAssign, shl_assign for Uint<U>, u32);

// ------------------------------ concrete types -------------------------------

generate_uint!(
    name = Uint64,
    inner_type = u64,
    from_int = [],
    from_std = [u32, u16, u8],
    doc = "64-bit unsigned integer.",
);

generate_uint!(
    name = Uint128,
    inner_type = u128,
    from_int = [Uint64],
    from_std = [u32, u16, u8],
    doc = "128-bit unsigned integer.",
);

generate_uint!(
    name = Uint256,
    inner_type = U256,
    from_int = [Uint64, Uint128],
    from_std = [u32, u16, u8],
    doc = "256-bit unsigned integer.",
);

generate_uint!(
    name = Uint512,
    inner_type = U512,
    from_int = [Uint256, Uint64, Uint128],
    from_std = [u32, u16, u8],
    doc = "512-bit unsigned integer.",
);

// TODO: can we merge these into `generate_uint`?
impl_next!(Uint64, Uint128);
impl_next!(Uint128, Uint256);
impl_next!(Uint256, Uint512);

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {super::*, crate::Dec128};

    /// Make sure we can't multiply a positive integer by a negative decimal.
    #[test]
    fn multiply_fraction_by_negative() {
        let lhs = Uint128::new(123);

        // Multiplying with a negative fraction should fail
        let rhs = Dec128::from_str("-0.1").unwrap();
        assert!(lhs.checked_mul_dec_floor(rhs).is_err());
        assert!(lhs.checked_mul_dec_ceil(rhs).is_err());
        assert!(lhs.checked_div_dec_floor(rhs).is_err());
        assert!(lhs.checked_div_dec_ceil(rhs).is_err());

        // Multiplying with negative zero is allowed though
        let rhs = Dec128::from_str("-0").unwrap();
        assert!(lhs.checked_mul_dec_floor(rhs).unwrap().is_zero());
        assert!(lhs.checked_mul_dec_ceil(rhs).unwrap().is_zero());

        // Dividing by zero should fail
        assert!(lhs.checked_div_dec_floor(rhs).is_err());
        assert!(lhs.checked_div_dec_ceil(rhs).is_err());
    }
}

#[cfg(test)]
mod tests2 {

    use super::*;

    macro_rules! inner_test {
        ([$($p:expr),*], $expr:expr) => {
            fn test<U, const B: usize>() where
                U: FromStr<Err: ToString> + NumberConst + Bytable<B>,
                {
                    ($expr)($($p),*);
                }
        };
    }

    macro_rules! utest {
        // Multiple args
        ($name:ident, [$($p64:expr),*], [$($p128:expr),*], [$($p256:expr),*], [$($p512:expr),*] => $expr:expr) => {
            paste::paste! {
                #[test]
                fn [<$name _u64 >]() {
                    inner_test!([$($p64),*], $expr);
                    test::<u64, 8>();
                }

                #[test]
                fn [<$name _u128 >]() {
                    inner_test!([$($p128),*], $expr);
                    test::<u128, 16>();
                }

                #[test]
                fn [<$name _u256 >]() {
                    inner_test!([$($p256),*], $expr);
                    test::<U256, 32>();
                }

                #[test]
                fn [<$name _u512 >]() {
                    inner_test!([$($p512),*], $expr);
                    test::<U512, 64>();
                }

            }

        };
    }

    utest!( size_of_works,
        [8],
        [16],
        [32],
        [64]
        => |size| {
            assert_eq!(core::mem::size_of::<Uint<U>>(), size);
        }
    );

    utest!( bytable_works,
        [&[0u8; 8]],
        [&[0u8; 16]],
        [&[0u8; 32]],
        [&[0u8; 64]]
        => |zero_as_byte: &[u8]| {
            let zero = Uint::<U>::ZERO;
            assert_eq!(zero.to_be_bytes().to_vec(), zero_as_byte);

            let one = Uint::<U>::ONE;
            let mut one_as_bytes: Vec<u8> = zero_as_byte.to_vec();
            if let Some(last) = one_as_bytes.last_mut() {
                *last = 1u8;
            }
            assert_eq!(one.to_be_bytes().to_vec(), one_as_bytes);

        }
    );

    utest!( convert_into,
        [12345u128],
        [12345u128],
        [12345u128],
        [12345u128]
        => |val| {
        //    let val: Uint::<U> = val.into();
        }
    );

    #[test]
    fn uint128_convert_into() {
        let original = Uint128(12345);
        let a = u128::from(original);
        assert_eq!(a, 12345);

        let original = Uint128(12345);
        let a = String::from(original);
        assert_eq!(a, "12345");
    }

    // #[test]
    // fn uint128_convert_from() {
    //     let a = Uint128::from(5u128);
    //     assert_eq!(a.0, 5);

    //     let a = Uint128::from(5u64);
    //     assert_eq!(a.0, 5);

    //     let a = Uint128::from(5u32);
    //     assert_eq!(a.0, 5);

    //     let a = Uint128::from(5u16);
    //     assert_eq!(a.0, 5);

    //     let a = Uint128::from(5u8);
    //     assert_eq!(a.0, 5);

    //     let result = Uint128::try_from("34567");
    //     assert_eq!(result.unwrap().0, 34567);

    //     let result = Uint128::try_from("1.23");
    //     assert!(result.is_err());
    // }

    // #[test]
    // fn uint128_try_from_signed_works() {
    //     test_try_from_int_to_uint::<Int64, Uint128>("Int64", "Uint128");
    //     test_try_from_int_to_uint::<Int128, Uint128>("Int128", "Uint128");
    //     test_try_from_int_to_uint::<Int256, Uint128>("Int256", "Uint128");
    //     test_try_from_int_to_uint::<Int512, Uint128>("Int512", "Uint128");
    // }

    // #[test]
    // fn uint128_try_into() {
    //     assert!(Uint64::try_from(Uint128::MAX).is_err());

    //     assert_eq!(Uint64::try_from(Uint128::zero()), Ok(Uint64::zero()));

    //     assert_eq!(
    //         Uint64::try_from(Uint128::from(42u64)),
    //         Ok(Uint64::from(42u64))
    //     );
    // }

    // #[test]
    // fn uint128_implements_display() {
    //     let a = Uint128(12345);
    //     assert_eq!(format!("Embedded: {a}"), "Embedded: 12345");
    //     assert_eq!(a.to_string(), "12345");

    //     let a = Uint128(0);
    //     assert_eq!(format!("Embedded: {a}"), "Embedded: 0");
    //     assert_eq!(a.to_string(), "0");
    // }

    // #[test]
    // fn uint128_display_padding_works() {
    //     // width > natural representation
    //     let a = Uint128::from(123u64);
    //     assert_eq!(format!("Embedded: {a:05}"), "Embedded: 00123");

    //     // width < natural representation
    //     let a = Uint128::from(123u64);
    //     assert_eq!(format!("Embedded: {a:02}"), "Embedded: 123");
    // }

    // #[test]
    // fn uint128_to_be_bytes_works() {
    //     assert_eq!(Uint128::zero().to_be_bytes(), [
    //         0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
    //     ]);
    //     assert_eq!(Uint128::MAX.to_be_bytes(), [
    //         0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    //         0xff, 0xff
    //     ]);
    //     assert_eq!(Uint128::new(1).to_be_bytes(), [
    //         0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1
    //     ]);
    //     // Python: `[b for b in (240282366920938463463374607431768124608).to_bytes(16, "big")]`
    //     assert_eq!(
    //         Uint128::new(240282366920938463463374607431768124608).to_be_bytes(),
    //         [180, 196, 179, 87, 165, 121, 59, 133, 246, 117, 221, 191, 255, 254, 172, 192]
    //     );
    // }

    // #[test]
    // fn uint128_to_le_bytes_works() {
    //     assert_eq!(Uint128::zero().to_le_bytes(), [
    //         0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
    //     ]);
    //     assert_eq!(Uint128::MAX.to_le_bytes(), [
    //         0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    //         0xff, 0xff
    //     ]);
    //     assert_eq!(Uint128::new(1).to_le_bytes(), [
    //         1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
    //     ]);
    //     // Python: `[b for b in (240282366920938463463374607431768124608).to_bytes(16, "little")]`
    //     assert_eq!(
    //         Uint128::new(240282366920938463463374607431768124608).to_le_bytes(),
    //         [192, 172, 254, 255, 191, 221, 117, 246, 133, 59, 121, 165, 87, 179, 196, 180]
    //     );
    // }

    // #[test]
    // fn uint128_is_zero_works() {
    //     assert!(Uint128::zero().is_zero());
    //     assert!(Uint128(0).is_zero());

    //     assert!(!Uint128(1).is_zero());
    //     assert!(!Uint128(123).is_zero());
    // }

    // #[test]
    // fn uint128_json() {
    //     let orig = Uint128(1234567890987654321);
    //     let serialized = serde_json::to_vec(&orig).unwrap();
    //     assert_eq!(serialized.as_slice(), b"\"1234567890987654321\"");
    //     let parsed: Uint128 = serde_json::from_slice(&serialized).unwrap();
    //     assert_eq!(parsed, orig);
    // }

    // #[test]
    // fn uint128_compare() {
    //     let a = Uint128(12345);
    //     let b = Uint128(23456);

    //     assert!(a < b);
    //     assert!(b > a);
    //     assert_eq!(a, Uint128(12345));
    // }

    // #[test]
    // #[allow(clippy::op_ref)]
    // fn uint128_math() {
    //     let a = Uint128(12345);
    //     let b = Uint128(23456);

    //     // test - with owned and reference right hand side
    //     assert_eq!(b - a, Uint128(11111));
    //     assert_eq!(b - &a, Uint128(11111));

    //     // test += with owned and reference right hand side
    //     let mut c = Uint128(300000);
    //     c += b;
    //     assert_eq!(c, Uint128(323456));
    //     let mut d = Uint128(300000);
    //     d += &b;
    //     assert_eq!(d, Uint128(323456));

    //     // test -= with owned and reference right hand side
    //     let mut c = Uint128(300000);
    //     c -= b;
    //     assert_eq!(c, Uint128(276544));
    //     let mut d = Uint128(300000);
    //     d -= &b;
    //     assert_eq!(d, Uint128(276544));

    //     // error result on underflow (- would produce negative result)
    //     let underflow_result = a.checked_sub(b);
    //     let OverflowError { operation } = underflow_result.unwrap_err();
    //     assert_eq!(operation, OverflowOperation::Sub);
    // }

    // #[test]
    // #[allow(clippy::op_ref)]
    // fn uint128_add_works() {
    //     assert_eq!(
    //         Uint128::from(2u32) + Uint128::from(1u32),
    //         Uint128::from(3u32)
    //     );
    //     assert_eq!(
    //         Uint128::from(2u32) + Uint128::from(0u32),
    //         Uint128::from(2u32)
    //     );

    //     // works for refs
    //     let a = Uint128::from(10u32);
    //     let b = Uint128::from(3u32);
    //     let expected = Uint128::from(13u32);
    //     assert_eq!(a + b, expected);
    //     assert_eq!(a + &b, expected);
    //     assert_eq!(&a + b, expected);
    //     assert_eq!(&a + &b, expected);
    // }

    // #[test]
    // #[should_panic(expected = "attempt to add with overflow")]
    // fn uint128_add_overflow_panics() {
    //     let max = Uint128::MAX;
    //     let _ = max + Uint128(12);
    // }

    // #[test]
    // #[allow(clippy::op_ref)]
    // fn uint128_sub_works() {
    //     assert_eq!(Uint128(2) - Uint128(1), Uint128(1));
    //     assert_eq!(Uint128(2) - Uint128(0), Uint128(2));
    //     assert_eq!(Uint128(2) - Uint128(2), Uint128(0));

    //     // works for refs
    //     let a = Uint128::new(10);
    //     let b = Uint128::new(3);
    //     let expected = Uint128::new(7);
    //     assert_eq!(a - b, expected);
    //     assert_eq!(a - &b, expected);
    //     assert_eq!(&a - b, expected);
    //     assert_eq!(&a - &b, expected);
    // }

    // #[test]
    // #[should_panic]
    // fn uint128_sub_overflow_panics() {
    //     let _ = Uint128(1) - Uint128(2);
    // }

    // #[test]
    // fn uint128_sub_assign_works() {
    //     let mut a = Uint128(14);
    //     a -= Uint128(2);
    //     assert_eq!(a, Uint128(12));

    //     // works for refs
    //     let mut a = Uint128::new(10);
    //     let b = Uint128::new(3);
    //     let expected = Uint128::new(7);
    //     a -= &b;
    //     assert_eq!(a, expected);
    // }

    // #[test]
    // #[allow(clippy::op_ref)]
    // fn uint128_mul_works() {
    //     assert_eq!(
    //         Uint128::from(2u32) * Uint128::from(3u32),
    //         Uint128::from(6u32)
    //     );
    //     assert_eq!(Uint128::from(2u32) * Uint128::zero(), Uint128::zero());

    //     // works for refs
    //     let a = Uint128::from(11u32);
    //     let b = Uint128::from(3u32);
    //     let expected = Uint128::from(33u32);
    //     assert_eq!(a * b, expected);
    //     assert_eq!(a * &b, expected);
    //     assert_eq!(&a * b, expected);
    //     assert_eq!(&a * &b, expected);
    // }

    // #[test]
    // fn uint128_mul_assign_works() {
    //     let mut a = Uint128::from(14u32);
    //     a *= Uint128::from(2u32);
    //     assert_eq!(a, Uint128::from(28u32));

    //     // works for refs
    //     let mut a = Uint128::from(10u32);
    //     let b = Uint128::from(3u32);
    //     a *= &b;
    //     assert_eq!(a, Uint128::from(30u32));
    // }

    // #[test]
    // fn uint128_pow_works() {
    //     assert_eq!(Uint128::from(2u32).pow(2), Uint128::from(4u32));
    //     assert_eq!(Uint128::from(2u32).pow(10), Uint128::from(1024u32));
    // }

    // #[test]
    // #[should_panic]
    // fn uint128_pow_overflow_panics() {
    //     _ = Uint128::MAX.pow(2u32);
    // }

    // #[test]
    // fn uint128_multiply_ratio_works() {
    //     let base = Uint128(500);

    //     // factor 1/1
    //     assert_eq!(base.multiply_ratio(1u128, 1u128), base);
    //     assert_eq!(base.multiply_ratio(3u128, 3u128), base);
    //     assert_eq!(base.multiply_ratio(654321u128, 654321u128), base);
    //     assert_eq!(base.multiply_ratio(u128::MAX, u128::MAX), base);

    //     // factor 3/2
    //     assert_eq!(base.multiply_ratio(3u128, 2u128), Uint128(750));
    //     assert_eq!(base.multiply_ratio(333333u128, 222222u128), Uint128(750));

    //     // factor 2/3 (integer devision always floors the result)
    //     assert_eq!(base.multiply_ratio(2u128, 3u128), Uint128(333));
    //     assert_eq!(base.multiply_ratio(222222u128, 333333u128), Uint128(333));

    //     // factor 5/6 (integer devision always floors the result)
    //     assert_eq!(base.multiply_ratio(5u128, 6u128), Uint128(416));
    //     assert_eq!(base.multiply_ratio(100u128, 120u128), Uint128(416));
    // }

    // #[test]
    // fn uint128_multiply_ratio_does_not_overflow_when_result_fits() {
    //     // Almost max value for Uint128.
    //     let base = Uint128(u128::MAX - 9);

    //     assert_eq!(base.multiply_ratio(2u128, 2u128), base);
    // }

    // #[test]
    // #[should_panic]
    // fn uint128_multiply_ratio_panicks_on_overflow() {
    //     // Almost max value for Uint128.
    //     let base = Uint128(u128::MAX - 9);

    //     assert_eq!(base.multiply_ratio(2u128, 1u128), base);
    // }

    // #[test]
    // #[should_panic(expected = "Denominator must not be zero")]
    // fn uint128_multiply_ratio_panics_for_zero_denominator() {
    //     _ = Uint128(500).multiply_ratio(1u128, 0u128);
    // }

    // #[test]
    // fn uint128_checked_multiply_ratio_does_not_panic() {
    //     assert_eq!(
    //         Uint128(500u128).checked_multiply_ratio(1u128, 0u128),
    //         Err(CheckedMultiplyRatioError::DivideByZero),
    //     );
    //     assert_eq!(
    //         Uint128(500u128).checked_multiply_ratio(u128::MAX, 1u128),
    //         Err(CheckedMultiplyRatioError::Overflow),
    //     );
    // }

    // #[test]
    // fn uint128_shr_works() {
    //     let original = Uint128::new(u128::from_be_bytes([
    //         0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 2u8, 0u8, 4u8, 2u8,
    //     ]));

    //     let shifted = Uint128::new(u128::from_be_bytes([
    //         0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 128u8, 1u8, 0u8,
    //     ]));

    //     assert_eq!(original >> 2u32, shifted);
    // }

    // #[test]
    // #[should_panic]
    // fn uint128_shr_overflow_panics() {
    //     let _ = Uint128::from(1u32) >> 128u32;
    // }

    // #[test]
    // fn uint128_shl_works() {
    //     let original = Uint128::new(u128::from_be_bytes([
    //         64u8, 128u8, 1u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
    //     ]));

    //     let shifted = Uint128::new(u128::from_be_bytes([
    //         2u8, 0u8, 4u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
    //     ]));

    //     assert_eq!(original << 2u32, shifted);
    // }

    // #[test]
    // #[should_panic]
    // fn uint128_shl_overflow_panics() {
    //     let _ = Uint128::from(1u32) << 128u32;
    // }

    // #[test]
    // fn sum_works() {
    //     let nums = vec![Uint128(17), Uint128(123), Uint128(540), Uint128(82)];
    //     let expected = Uint128(762);

    //     let sum_as_ref: Uint128 = nums.iter().sum();
    //     assert_eq!(expected, sum_as_ref);

    //     let sum_as_owned: Uint128 = nums.into_iter().sum();
    //     assert_eq!(expected, sum_as_owned);
    // }

    // #[test]
    // fn uint128_methods() {
    //     // checked_*
    //     assert!(matches!(
    //         Uint128::MAX.checked_add(Uint128(1)),
    //         Err(OverflowError { .. })
    //     ));
    //     assert!(matches!(Uint128(1).checked_add(Uint128(1)), Ok(Uint128(2))));
    //     assert!(matches!(
    //         Uint128(0).checked_sub(Uint128(1)),
    //         Err(OverflowError { .. })
    //     ));
    //     assert!(matches!(Uint128(2).checked_sub(Uint128(1)), Ok(Uint128(1))));
    //     assert!(matches!(
    //         Uint128::MAX.checked_mul(Uint128(2)),
    //         Err(OverflowError { .. })
    //     ));
    //     assert!(matches!(Uint128(2).checked_mul(Uint128(2)), Ok(Uint128(4))));
    //     assert!(matches!(
    //         Uint128::MAX.checked_pow(2u32),
    //         Err(OverflowError { .. })
    //     ));
    //     assert!(matches!(Uint128(2).checked_pow(3), Ok(Uint128(8))));
    //     assert!(matches!(
    //         Uint128::MAX.checked_div(Uint128(0)),
    //         Err(DivideByZeroError { .. })
    //     ));
    //     assert!(matches!(Uint128(6).checked_div(Uint128(2)), Ok(Uint128(3))));
    //     assert!(matches!(
    //         Uint128::MAX.checked_div_euclid(Uint128(0)),
    //         Err(DivideByZeroError { .. })
    //     ));
    //     assert!(matches!(
    //         Uint128(6).checked_div_euclid(Uint128(2)),
    //         Ok(Uint128(3)),
    //     ));
    //     assert!(matches!(
    //         Uint128::MAX.checked_rem(Uint128(0)),
    //         Err(DivideByZeroError { .. })
    //     ));

    //     // saturating_*
    //     assert_eq!(Uint128::MAX.saturating_add(Uint128(1)), Uint128::MAX);
    //     assert_eq!(Uint128(0).saturating_sub(Uint128(1)), Uint128(0));
    //     assert_eq!(Uint128::MAX.saturating_mul(Uint128(2)), Uint128::MAX);
    //     assert_eq!(Uint128::MAX.saturating_pow(2), Uint128::MAX);
    // }

    // #[test]
    // fn uint128_wrapping_methods() {
    //     // wrapping_add
    //     assert_eq!(Uint128(2).wrapping_add(Uint128(2)), Uint128(4)); // non-wrapping
    //     assert_eq!(Uint128::MAX.wrapping_add(Uint128(1)), Uint128(0)); // wrapping

    //     // wrapping_sub
    //     assert_eq!(Uint128(7).wrapping_sub(Uint128(5)), Uint128(2)); // non-wrapping
    //     assert_eq!(Uint128(0).wrapping_sub(Uint128(1)), Uint128::MAX); // wrapping

    //     // wrapping_mul
    //     assert_eq!(Uint128(3).wrapping_mul(Uint128(2)), Uint128(6)); // non-wrapping
    //     assert_eq!(
    //         Uint128::MAX.wrapping_mul(Uint128(2)),
    //         Uint128::MAX - Uint128::one()
    //     ); // wrapping

    //     // wrapping_pow
    //     assert_eq!(Uint128(2).wrapping_pow(3), Uint128(8)); // non-wrapping
    //     assert_eq!(Uint128::MAX.wrapping_pow(2), Uint128(1)); // wrapping
    // }

    // #[test]
    // #[allow(clippy::op_ref)]
    // fn uint128_implements_rem() {
    //     let a = Uint128::new(10);
    //     assert_eq!(a % Uint128::new(10), Uint128::zero());
    //     assert_eq!(a % Uint128::new(2), Uint128::zero());
    //     assert_eq!(a % Uint128::new(1), Uint128::zero());
    //     assert_eq!(a % Uint128::new(3), Uint128::new(1));
    //     assert_eq!(a % Uint128::new(4), Uint128::new(2));

    //     // works for refs
    //     let a = Uint128::new(10);
    //     let b = Uint128::new(3);
    //     let expected = Uint128::new(1);
    //     assert_eq!(a % b, expected);
    //     assert_eq!(a % &b, expected);
    //     assert_eq!(&a % b, expected);
    //     assert_eq!(&a % &b, expected);
    // }

    // #[test]
    // #[should_panic(expected = "divisor of zero")]
    // fn uint128_rem_panics_for_zero() {
    //     let _ = Uint128::new(10) % Uint128::zero();
    // }

    // #[test]
    // #[allow(clippy::op_ref)]
    // fn uint128_rem_works() {
    //     assert_eq!(
    //         Uint128::from(12u32) % Uint128::from(10u32),
    //         Uint128::from(2u32)
    //     );
    //     assert_eq!(Uint128::from(50u32) % Uint128::from(5u32), Uint128::zero());

    //     // works for refs
    //     let a = Uint128::from(42u32);
    //     let b = Uint128::from(5u32);
    //     let expected = Uint128::from(2u32);
    //     assert_eq!(a % b, expected);
    //     assert_eq!(a % &b, expected);
    //     assert_eq!(&a % b, expected);
    //     assert_eq!(&a % &b, expected);
    // }

    // #[test]
    // fn uint128_rem_assign_works() {
    //     let mut a = Uint128::from(30u32);
    //     a %= Uint128::from(4u32);
    //     assert_eq!(a, Uint128::from(2u32));

    //     // works for refs
    //     let mut a = Uint128::from(25u32);
    //     let b = Uint128::from(6u32);
    //     a %= &b;
    //     assert_eq!(a, Uint128::from(1u32));
    // }

    // #[test]
    // fn uint128_strict_add_works() {
    //     let a = Uint128::new(5);
    //     let b = Uint128::new(3);
    //     assert_eq!(a.strict_add(b), Uint128::new(8));
    //     assert_eq!(b.strict_add(a), Uint128::new(8));
    // }

    // #[test]
    // #[should_panic(expected = "attempt to add with overflow")]
    // fn uint128_strict_add_panics_on_overflow() {
    //     let a = Uint128::MAX;
    //     let b = Uint128::ONE;
    //     let _ = a.strict_add(b);
    // }

    // #[test]
    // fn uint128_strict_sub_works() {
    //     let a = Uint128::new(5);
    //     let b = Uint128::new(3);
    //     assert_eq!(a.strict_sub(b), Uint128::new(2));
    // }

    // #[test]
    // #[should_panic(expected = "attempt to subtract with overflow")]
    // fn uint128_strict_sub_panics_on_overflow() {
    //     let a = Uint128::ZERO;
    //     let b = Uint128::ONE;
    //     let _ = a.strict_sub(b);
    // }

    // #[test]
    // fn uint128_abs_diff_works() {
    //     let a = Uint128::from(42u32);
    //     let b = Uint128::from(5u32);
    //     let expected = Uint128::from(37u32);
    //     assert_eq!(a.abs_diff(b), expected);
    //     assert_eq!(b.abs_diff(a), expected);
    // }

    // #[test]
    // fn uint128_partial_eq() {
    //     let test_cases = [(1, 1, true), (42, 42, true), (42, 24, false), (0, 0, true)]
    //         .into_iter()
    //         .map(|(lhs, rhs, expected)| (Uint128::new(lhs), Uint128::new(rhs), expected));

    //     #[allow(clippy::op_ref)]
    //     for (lhs, rhs, expected) in test_cases {
    //         assert_eq!(lhs == rhs, expected);
    //         assert_eq!(&lhs == rhs, expected);
    //         assert_eq!(lhs == &rhs, expected);
    //         assert_eq!(&lhs == &rhs, expected);
    //     }
    // }

    // #[test]
    // fn mul_floor_works_with_zero() {
    //     let fraction = (Uint128::zero(), Uint128::new(21));
    //     let res = Uint128::new(123456).mul_floor(fraction);
    //     assert_eq!(Uint128::zero(), res)
    // }

    // #[test]
    // fn mul_floor_does_nothing_with_one() {
    //     let fraction = (Uint128::one(), Uint128::one());
    //     let res = Uint128::new(123456).mul_floor(fraction);
    //     assert_eq!(Uint128::new(123456), res)
    // }

    // #[test]
    // fn mul_floor_rounds_down_with_normal_case() {
    //     let fraction = (8u128, 21u128);
    //     let res = Uint128::new(123456).mul_floor(fraction); // 47030.8571
    //     assert_eq!(Uint128::new(47030), res)
    // }

    // #[test]
    // fn mul_floor_does_not_round_on_even_divide() {
    //     let fraction = (2u128, 5u128);
    //     let res = Uint128::new(25).mul_floor(fraction);
    //     assert_eq!(Uint128::new(10), res)
    // }

    // #[test]
    // fn mul_floor_works_when_operation_temporarily_takes_above_max() {
    //     let fraction = (8u128, 21u128);
    //     let res = Uint128::MAX.mul_floor(fraction); // 129_631_377_874_643_224_176_523_659_974_006_937_697.14285
    //     assert_eq!(
    //         Uint128::new(129_631_377_874_643_224_176_523_659_974_006_937_697),
    //         res
    //     )
    // }

    // #[test]
    // fn mul_floor_works_with_decimal() {
    //     let decimal = Decimal::from_ratio(8u128, 21u128);
    //     let res = Uint128::new(123456).mul_floor(decimal); // 47030.8571
    //     assert_eq!(Uint128::new(47030), res)
    // }

    // #[test]
    // #[should_panic(expected = "ConversionOverflowError")]
    // fn mul_floor_panics_on_overflow() {
    //     let fraction = (21u128, 8u128);
    //     _ = Uint128::MAX.mul_floor(fraction);
    // }

    // #[test]
    // fn checked_mul_floor_does_not_panic_on_overflow() {
    //     let fraction = (21u128, 8u128);
    //     assert_eq!(
    //         Uint128::MAX.checked_mul_floor(fraction),
    //         Err(ConversionOverflow(ConversionOverflowError {
    //             source_type: "Uint256",
    //             target_type: "Uint128",
    //         })),
    //     );
    // }

    // #[test]
    // #[should_panic(expected = "DivideByZeroError")]
    // fn mul_floor_panics_on_zero_div() {
    //     let fraction = (21u128, 0u128);
    //     _ = Uint128::new(123456).mul_floor(fraction);
    // }

    // #[test]
    // fn checked_mul_floor_does_not_panic_on_zero_div() {
    //     let fraction = (21u128, 0u128);
    //     assert_eq!(
    //         Uint128::new(123456).checked_mul_floor(fraction),
    //         Err(DivideByZero(DivideByZeroError)),
    //     );
    // }

    // #[test]
    // fn mul_ceil_works_with_zero() {
    //     let fraction = (Uint128::zero(), Uint128::new(21));
    //     let res = Uint128::new(123456).mul_ceil(fraction);
    //     assert_eq!(Uint128::zero(), res)
    // }

    // #[test]
    // fn mul_ceil_does_nothing_with_one() {
    //     let fraction = (Uint128::one(), Uint128::one());
    //     let res = Uint128::new(123456).mul_ceil(fraction);
    //     assert_eq!(Uint128::new(123456), res)
    // }

    // #[test]
    // fn mul_ceil_rounds_up_with_normal_case() {
    //     let fraction = (8u128, 21u128);
    //     let res = Uint128::new(123456).mul_ceil(fraction); // 47030.8571
    //     assert_eq!(Uint128::new(47031), res)
    // }

    // #[test]
    // fn mul_ceil_does_not_round_on_even_divide() {
    //     let fraction = (2u128, 5u128);
    //     let res = Uint128::new(25).mul_ceil(fraction);
    //     assert_eq!(Uint128::new(10), res)
    // }

    // #[test]
    // fn mul_ceil_works_when_operation_temporarily_takes_above_max() {
    //     let fraction = (8u128, 21u128);
    //     let res = Uint128::MAX.mul_ceil(fraction); // 129_631_377_874_643_224_176_523_659_974_006_937_697.14285
    //     assert_eq!(
    //         Uint128::new(129_631_377_874_643_224_176_523_659_974_006_937_698),
    //         res
    //     )
    // }

    // #[test]
    // fn mul_ceil_works_with_decimal() {
    //     let decimal = Decimal::from_ratio(8u128, 21u128);
    //     let res = Uint128::new(123456).mul_ceil(decimal); // 47030.8571
    //     assert_eq!(Uint128::new(47031), res)
    // }

    // #[test]
    // #[should_panic(expected = "ConversionOverflowError")]
    // fn mul_ceil_panics_on_overflow() {
    //     let fraction = (21u128, 8u128);
    //     _ = Uint128::MAX.mul_ceil(fraction);
    // }

    // #[test]
    // fn checked_mul_ceil_does_not_panic_on_overflow() {
    //     let fraction = (21u128, 8u128);
    //     assert_eq!(
    //         Uint128::MAX.checked_mul_ceil(fraction),
    //         Err(ConversionOverflow(ConversionOverflowError {
    //             source_type: "Uint256",
    //             target_type: "Uint128",
    //         })),
    //     );
    // }

    // #[test]
    // #[should_panic(expected = "DivideByZeroError")]
    // fn mul_ceil_panics_on_zero_div() {
    //     let fraction = (21u128, 0u128);
    //     _ = Uint128::new(123456).mul_ceil(fraction);
    // }

    // #[test]
    // fn checked_mul_ceil_does_not_panic_on_zero_div() {
    //     let fraction = (21u128, 0u128);
    //     assert_eq!(
    //         Uint128::new(123456).checked_mul_ceil(fraction),
    //         Err(DivideByZero(DivideByZeroError)),
    //     );
    // }

    // #[test]
    // #[should_panic(expected = "DivideByZeroError")]
    // fn div_floor_raises_with_zero() {
    //     let fraction = (Uint128::zero(), Uint128::new(21));
    //     _ = Uint128::new(123456).div_floor(fraction);
    // }

    // #[test]
    // fn div_floor_does_nothing_with_one() {
    //     let fraction = (Uint128::one(), Uint128::one());
    //     let res = Uint128::new(123456).div_floor(fraction);
    //     assert_eq!(Uint128::new(123456), res)
    // }

    // #[test]
    // fn div_floor_rounds_down_with_normal_case() {
    //     let fraction = (5u128, 21u128);
    //     let res = Uint128::new(123456).div_floor(fraction); // 518515.2
    //     assert_eq!(Uint128::new(518515), res)
    // }

    // #[test]
    // fn div_floor_does_not_round_on_even_divide() {
    //     let fraction = (5u128, 2u128);
    //     let res = Uint128::new(25).div_floor(fraction);
    //     assert_eq!(Uint128::new(10), res)
    // }

    // #[test]
    // fn div_floor_works_when_operation_temporarily_takes_above_max() {
    //     let fraction = (21u128, 8u128);
    //     let res = Uint128::MAX.div_floor(fraction); // 129_631_377_874_643_224_176_523_659_974_006_937_697.1428
    //     assert_eq!(
    //         Uint128::new(129_631_377_874_643_224_176_523_659_974_006_937_697),
    //         res
    //     )
    // }

    // #[test]
    // fn div_floor_works_with_decimal() {
    //     let decimal = Decimal::from_ratio(21u128, 8u128);
    //     let res = Uint128::new(123456).div_floor(decimal); // 47030.8571
    //     assert_eq!(Uint128::new(47030), res)
    // }

    // #[test]
    // fn div_floor_works_with_decimal_evenly() {
    //     let res = Uint128::new(60).div_floor(Decimal::from_atomics(6u128, 0).unwrap());
    //     assert_eq!(res, Uint128::new(10));
    // }

    // #[test]
    // #[should_panic(expected = "ConversionOverflowError")]
    // fn div_floor_panics_on_overflow() {
    //     let fraction = (8u128, 21u128);
    //     _ = Uint128::MAX.div_floor(fraction);
    // }

    // #[test]
    // fn div_floor_does_not_panic_on_overflow() {
    //     let fraction = (8u128, 21u128);
    //     assert_eq!(
    //         Uint128::MAX.checked_div_floor(fraction),
    //         Err(ConversionOverflow(ConversionOverflowError {
    //             source_type: "Uint256",
    //             target_type: "Uint128",
    //         })),
    //     );
    // }

    // #[test]
    // #[should_panic(expected = "DivideByZeroError")]
    // fn div_ceil_raises_with_zero() {
    //     let fraction = (Uint128::zero(), Uint128::new(21));
    //     _ = Uint128::new(123456).div_ceil(fraction);
    // }

    // #[test]
    // fn div_ceil_does_nothing_with_one() {
    //     let fraction = (Uint128::one(), Uint128::one());
    //     let res = Uint128::new(123456).div_ceil(fraction);
    //     assert_eq!(Uint128::new(123456), res)
    // }

    // #[test]
    // fn div_ceil_rounds_up_with_normal_case() {
    //     let fraction = (5u128, 21u128);
    //     let res = Uint128::new(123456).div_ceil(fraction); // 518515.2
    //     assert_eq!(Uint128::new(518516), res)
    // }

    // #[test]
    // fn div_ceil_does_not_round_on_even_divide() {
    //     let fraction = (5u128, 2u128);
    //     let res = Uint128::new(25).div_ceil(fraction);
    //     assert_eq!(Uint128::new(10), res)
    // }

    // #[test]
    // fn div_ceil_works_when_operation_temporarily_takes_above_max() {
    //     let fraction = (21u128, 8u128);
    //     let res = Uint128::MAX.div_ceil(fraction); // 129_631_377_874_643_224_176_523_659_974_006_937_697.1428
    //     assert_eq!(
    //         Uint128::new(129_631_377_874_643_224_176_523_659_974_006_937_698),
    //         res
    //     )
    // }

    // #[test]
    // fn div_ceil_works_with_decimal() {
    //     let decimal = Decimal::from_ratio(21u128, 8u128);
    //     let res = Uint128::new(123456).div_ceil(decimal); // 47030.8571
    //     assert_eq!(Uint128::new(47031), res)
    // }

    // #[test]
    // fn div_ceil_works_with_decimal_evenly() {
    //     let res = Uint128::new(60).div_ceil(Decimal::from_atomics(6u128, 0).unwrap());
    //     assert_eq!(res, Uint128::new(10));
    // }

    // #[test]
    // #[should_panic(expected = "ConversionOverflowError")]
    // fn div_ceil_panics_on_overflow() {
    //     let fraction = (8u128, 21u128);
    //     _ = Uint128::MAX.div_ceil(fraction);
    // }

    // #[test]
    // fn div_ceil_does_not_panic_on_overflow() {
    //     let fraction = (8u128, 21u128);
    //     assert_eq!(
    //         Uint128::MAX.checked_div_ceil(fraction),
    //         Err(ConversionOverflow(ConversionOverflowError {
    //             source_type: "Uint256",
    //             target_type: "Uint128",
    //         })),
    //     );
    // }
}
