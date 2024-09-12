use {
    crate::{
        forward_ref_binop_typed, forward_ref_op_assign_typed, forward_ref_partial_eq,
        generate_uint, impl_all_ops_and_assign, impl_assign_integer, impl_assign_number,
        impl_integer, impl_next, impl_number, Bytable, Fraction, Inner, Integer, MultiplyFraction,
        MultiplyRatio, NextNumber, Number, NumberConst, Sign, StdError, StdResult,
    },
    bnum::types::{U256, U512},
    borsh::{BorshDeserialize, BorshSerialize},
    forward_ref::{forward_ref_binop, forward_ref_op_assign},
    serde::{de, ser},
    std::{
        fmt::{self, Display},
        iter::Sum,
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
        let denominator: Self = denominator.into();
        let dividend = self.checked_full_mul(numerator)?;
        let floor_result = self.checked_multiply_ratio_floor(numerator, denominator)?;
        let remained = dividend.checked_rem(denominator.into_next())?;
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

impl<U, A> Sum<A> for Uint<U>
where
    Self: Add<A, Output = Self>,
    U: Number + NumberConst,
{
    fn sum<I: Iterator<Item = A>>(iter: I) -> Self {
        iter.fold(Self::ZERO, Add::add)
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

// forward_ref_partial_eq_typed!(impl<U> Uint<U>,  Uint<U>);

impl_number!(impl<U> Add, add for Uint<U> where sub fn checked_add);
impl_number!(impl<U> Sub, sub for Uint<U> where sub fn checked_sub);
impl_number!(impl<U> Mul, mul for Uint<U> where sub fn checked_mul);
impl_number!(impl<U> Div, div for Uint<U> where sub fn checked_div);
impl_number!(impl<U> Rem, rem for Uint<U> where sub fn checked_rem);
impl_integer!(impl<U> Shl, shl for Uint<U> where sub fn checked_shl, u32);
impl_integer!(impl<U> Shr, shr for Uint<U> where sub fn checked_shr, u32);

impl_assign_number!(impl<U> AddAssign, add_assign for Uint<U> where sub fn checked_add);
impl_assign_number!(impl<U> SubAssign, sub_assign for Uint<U> where sub fn checked_sub);
impl_assign_number!(impl<U> MulAssign, mul_assign for Uint<U> where sub fn checked_mul);
impl_assign_number!(impl<U> DivAssign, div_assign for Uint<U> where sub fn checked_div);
impl_assign_number!(impl<U> RemAssign, rem_assign for Uint<U> where sub fn checked_rem);
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
forward_ref_op_assign_typed!(impl<U> RemAssign, rem_assign for Uint<U>, Uint<U>);
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
#[allow(clippy::just_underscores_and_digits)]
mod tests {

    use {
        super::*,
        crate::{Dec128, Dec256, Int128, Int256, Int64, Udec128, Udec256},
        fmt::Debug,
    };

    /// `derive_type`
    ///
    /// Allow compiler to derive the type of a variable,
    /// which is necessary for the test functions.
    fn dt<T>(_: T, _: T) {}

    /// `built_type`
    ///
    ///  Allow compiler to derive the type of a variable, and return right.
    fn bt<T>(_: T, ret: T) -> T {
        ret
    }

    /// `derive_types`
    ///
    ///  Allow compiler to derive the types of multiple variables
    macro_rules! dts{
        ($u: expr, $($p:expr),* ) =>
         {
            $(dt($u, $p);)*
         }
    }

    /// Combines `assert_eq` and `derive_type` to derive the type and assert
    fn smart_assert<T: Debug + PartialEq>(left: T, right: T) {
        assert_eq!(left, right);
    }

    /// Macro for unit tests for Uint.
    /// Is not possible to use [`test_case::test_case`] because the arguments types can are different.
    /// Also `Uint<U>` is different for each test case.
    ///
    /// The macro set as first parameter of the callback function `Uint::ZERO`, so the compiler can derive the type
    /// (see [`derive_type`], [`derive_types`] and [`smart_assert`] ).
    macro_rules! utest {
        // Multiple args
        (
            $name:ident,
            [$($p64:expr),*],
            [$($p128:expr),*],
            [$($p256:expr),*],
            [$($p512:expr),*]
            $(attrs = $(#[$meta:meta])*)?
            => $test_fn:expr) => {
            paste::paste! {
                #[test]
                $($(#[$meta])*)?
                fn [<$name _u64 >]() {
                    // the first argument is used to derive the type of the variable
                    ($test_fn)(Uint64::ZERO, $($p64),*);
                }

                #[test]
                $($(#[$meta])*)?
                fn [<$name _u128 >]() {
                    ($test_fn)(Uint128::ZERO, $($p128),*);
                }

                #[test]
                $($(#[$meta])*)?
                fn [<$name _u256 >]() {
                    ($test_fn)(Uint256::ZERO, $($p256),*);
                }

                #[test]
                $($(#[$meta])*)?
                fn [<$name _u512 >]() {
                    ($test_fn)(Uint512::ZERO, $($p512),*);
                }
            }
        };
        // No args
        (
            $name:ident,
            $(attrs = $(#[$meta:meta])*)?
            => $test_fn:expr) => {
                utest!($name, [], [], [], [] $(attrs = $(#[$meta])*)? => $test_fn);
        };
        // Same args
        (
            $name:ident,
            [$($p:expr),*]
            $(attrs = $(#[$meta:meta])*)?
            => $test_fn:expr) => {
                utest!($name, [$($p),*], [$($p),*], [$($p),*], [$($p),*] $(attrs = $(#[$meta])*)? => $test_fn);
        };
        // Multiple optional tests.
        // is not possible to use `$(attrs = $(#[$meta:meta])*)?`
        // because it seems not possible to have nested optional arguments
        (
            $name:ident,
            $(64 = [$($p64:expr),*])?
            $(128 = [$($p128:expr),*])?
            $(256 = [$($p256:expr),*])?
            $(512 = [$($p512:expr),*])?
            => $test_fn:expr) => {
            paste::paste! {
                $(
                    #[test]
                    fn [<$name _u64>]() {
                        ($test_fn)(Uint64::ZERO, $($p64),*);
                    }
                )?

                $(
                    #[test]
                    fn [<$name _u128 >]() {
                        ($test_fn)(Uint128::ZERO, $($p128),*);
                    }
                )?

                $(
                    #[test]
                    fn [<$name _u256 >]() {
                        ($test_fn)(Uint256::ZERO, $($p256),*);
                    }
                )?

                $(
                    #[test]
                    fn [<$name _u512 >]() {
                        ($test_fn)(Uint512::ZERO, $($p512),*);
                    }
                )?
            }
        };

    }

    utest!( size_of,
        [8],
        [16],
        [32],
        [64]
        => |_0, size| {
            assert_eq!(core::mem::size_of_val(&_0), size);
        }
    );

    utest!( bytable_to_be,
        [&[0u8; 8],  &[0xff; 8]],
        [&[0u8; 16], &[0xff; 16]],
        [&[0u8; 32], &[0xff; 32]],
        [&[0u8; 64], &[0xff; 64]]
        => |_0, zero_as_byte: &[u8], max_as_byte| {
            let _1 = Uint::ONE;
            let max = Uint::MAX;
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

    utest!( bytable_to_le,
        [&[0u8; 8],  &[0xff; 8]],
        [&[0u8; 16], &[0xff; 16]],
        [&[0u8; 32], &[0xff; 32]],
        [&[0u8; 64], &[0xff; 64]]
        => |_0, zero_as_byte: &[u8], max_as_byte| {
            let _1 = Uint::ONE;
            let max = Uint::MAX;
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

    utest!( converts,
        [64_u64,                "64"],
        [128_u128,             "128"],
        [U256::from(256_u128), "256"],
        [U512::from(512_u128), "512"]
        => |_, val, str| {
           let original = Uint::new(val);
           assert_eq!(original.0, val);

           let from_str = Uint::from_str(str).unwrap();
           assert_eq!(from_str, original);

           let as_into = original.into();
           dt(as_into, val);

           assert_eq!(as_into, val);
        }
    );

    utest!( from,
        [8_u8, 16_u16, 32_u32, None::<u64> , None::<u64> , None::<u64>],
        [8_u8, 16_u16, 32_u32, Some(64_u64), None::<u128> , None::<u128>],
        [8_u8, 16_u16, 32_u32, Some(64_u64), Some(128_u128), None::<U256>],
        [8_u8, 16_u16, 32_u32, Some(64_u64), Some(128_u128), Some(U256::from(256_u128))]
        => |_0, u8, u16, u32, u64, u128, u256| {
            let uint8 = Uint::from(u8);
            let uint16 = Uint::from(u16);
            let uint32 = Uint::from(u32);

            dts!(_0, uint8, uint16, uint32);

            smart_assert(u8, uint8.try_into().unwrap());
            smart_assert(u16, uint16.try_into().unwrap());
            smart_assert(u32, uint32.try_into().unwrap());

            macro_rules! maybe_from {
                ($t:expr) => {
                    if let Some(t) = $t {
                        let uint = bt(_0, Uint::from(t));
                        smart_assert(t, uint.try_into().unwrap());
                    }
                };
            }

            maybe_from!(u64);
            maybe_from!(u128);
            maybe_from!(u256);
        }
    );

    utest!( try_from_signed,
        64  = [Int64::new_positive(64_u64.into()),    Int64::new_negative(64_u64.into())]
        128 = [Int128::new_positive(128_u128.into()), Int128::new_negative(128_u128.into())]
        256 = [Int256::new_positive(256_u128.into()), Int256::new_negative(256_u128.into())]
        // We don't have Int512
        => |_0, positive, negative| {
            let uint = Uint::try_from(positive).unwrap();
            dt(uint, _0);

            let maybe_uint = Uint::try_from(negative);
            dt(&maybe_uint, &Ok(_0));
            maybe_uint.unwrap_err();
        }
    );

    utest!( try_into,
       64  = [Some(Uint128::MAX), Uint128::ZERO, Uint128::from(64_u128), Uint64::from(64_u64)]
       128 = [Some(Uint256::MAX), Uint256::ZERO, Uint256::from(128_u128), Uint128::from(128_u128)]
       256 = [Some(Uint512::MAX), Uint512::ZERO, Uint512::from(256_u128), Uint256::from(256_u128)]
       => |_0, next_max, next_zero, next_valid, compare| {

            if let Some(next_max) = next_max {
                let maybe_uint = Uint::try_from(next_max);
                dt(&maybe_uint, &Ok(_0));
                maybe_uint.unwrap_err();
            }

            let uint_zero = Uint::try_from(next_zero).unwrap();
            assert_eq!(_0, uint_zero);

            let uint = Uint::try_from(next_valid).unwrap();
            assert_eq!(uint, compare);

        }
    );

    utest!( display,
        [Uint64::new(64_u64), "64"],
        [Uint128::new(128_u128), "128"],
        [Uint256::new(U256::from(256_u128)), "256"],
        [Uint512::new(U512::from(512_u128)), "512"]
        => |_, uint, str| {
            assert_eq!(format!("{}", uint), str);
        }
    );

    utest!( display_padding_front,
        ["064", "64"],
        ["00128", "128"],
        ["000256", "256"],
        ["0000512", "512"]
        => |_0, padded_str, compare| {
            let uint = bt(_0, Uint::from_str(padded_str).unwrap());
            assert_eq!(format!("{}", uint), compare);
        }
    );

    utest!( is_zero,
        => |zero: Uint<_>| {
            assert!(zero.is_zero());

            let non_zero = Uint::ONE;
            dt(non_zero, zero);
            assert!(!non_zero.is_zero());
        }
    );

    utest!( json,
    => |_0| {
        let original = bt(_0, Uint::from_str("123456").unwrap());

        let serialized_str = serde_json::to_string(&original).unwrap();
        assert_eq!(serialized_str, format!("\"{}\"", "123456"));

        let serialized_vec = serde_json::to_vec(&original).unwrap();
        assert_eq!(serialized_vec, format!("\"{}\"", "123456").as_bytes());

        let parsed: Uint::<_> = serde_json::from_str(&serialized_str).unwrap();
        assert_eq!(parsed, original);

        let parsed: Uint::<_> = serde_json::from_slice(&serialized_vec).unwrap();
        assert_eq!(parsed, original);
    });

    utest!( compare,
        => |_0| {
            let a = Uint::from(10_u64);
            let b = Uint::from(20_u64);
            dts!(_0, a, b);

            assert!(a < b);
            assert!(b > a);
            assert_eq!(a, a);
        }
    );

    utest!( math,
        attrs = #[allow(clippy::op_ref)]
        => |_0| {
            let a = Uint::from(12345_u64);
            let b = Uint::from(23456_u64);
            dts!(_0, a, b);

            // test - with owned and reference right hand side
            let diff = bt(_0, Uint::from(11111_u64));
            assert_eq!(b - a, diff);
            assert_eq!(b - &a, diff);

            // test += with owned and reference right hand side
            let mut c = bt(_0, Uint::from(300000_u64));
            c += b;
            assert_eq!(c, bt(_0, Uint::from(323456_u64)));

            let mut d = bt(_0, Uint::from(300000_u64));
            d += &b;
            assert_eq!(d,  bt(_0, Uint::from(323456_u64)));

            // test -= with owned and reference right hand side
            let mut c = bt(_0, Uint::from(300000_u64));
            c -= b;
            assert_eq!(c, bt(_0, Uint::from(276544_u64)));
            let mut d = bt(_0, Uint::from(300000_u64));
            d -= &b;
            assert_eq!(d, bt(_0, Uint::from(276544_u64)));

            // error result on underflow (- would produce negative result)
            let underflow_result = a.checked_sub(b);
            let StdError::OverflowSub { .. } = underflow_result.unwrap_err() else {
                panic!("Expected OverflowSub error");
            };
        }
    );

    utest!( add,
        attrs = #[allow(clippy::op_ref)]
        => |_0| {
            assert_eq!(
                bt(_0, Uint::from(2_u64)) + bt(_0, Uint::from(1_u64)),
                bt(_0, Uint::from(3_u64))
            );
            assert_eq!(
                bt(_0, Uint::from(2_u64)) + bt(_0, Uint::from(0_u64)),
                bt(_0, Uint::from(2_u64))
            );

            let a = bt(_0, Uint::from(10_u64));
            let b = bt(_0, Uint::from(3_u64));
            let expected = bt(_0, Uint::from(13_u64));
            assert_eq!(a + b, expected);
            assert_eq!(a + &b, expected);
            assert_eq!(&a + b, expected);
            assert_eq!(&a + &b, expected);

        }
    );

    utest!( add_overflow_panics,
        attrs = #[should_panic(expected = "addition overflow")]
        => |_0| {
            let max = bt(_0, Uint::MAX);
            let _ = max + bt(_0, Uint::from(12_u64));
        }
    );

    utest!( sub,
        attrs = #[allow(clippy::op_ref)]
        => |_0| {
            assert_eq!(bt(_0, Uint::from(2_u64)) - bt(_0, Uint::from(1_u64)), bt(_0, Uint::from(1_u64)));
            assert_eq!(bt(_0, Uint::from(2_u64)) - bt(_0, Uint::from(0_u64)), bt(_0, Uint::from(2_u64)));
            assert_eq!(bt(_0, Uint::from(2_u64)) - bt(_0, Uint::from(2_u64)), bt(_0, Uint::from(0_u64)));

            // works for refs
            let a = Uint::from(10_u64);
            let b = Uint::from(3_u64);
            let expected = Uint::from(7_u64);
            dts!(_0, a, b, expected);
            assert_eq!(a - b, expected);
            assert_eq!(a - &b, expected);
            assert_eq!(&a - b, expected);
            assert_eq!(&a - &b, expected);
        }
    );

    utest!( sub_overflow_panics,
        attrs = #[should_panic(expected = "subtraction overflow")]
        => |_0| {
            let _ = bt(_0, Uint::from(1_u64)) - bt(_0, Uint::from(2_u64));
        }
    );

    utest!( sub_assign_works,
        attrs = #[allow(clippy::op_ref)]
        => |_0| {
            let mut a = bt(_0, Uint::from(14_u64));
            a -= bt(_0, Uint::from(2_u64));
            assert_eq!(a, bt(_0, Uint::from(12_u64)));

            // works for refs
            let mut a = bt(_0, Uint::from(10_u64));
            let b = bt(_0, Uint::from(3_u64));
            let expected = bt(_0, Uint::from(7_u64));
            a -= &b;
            assert_eq!(a, expected);
        }
    );

    utest!( mul,
        attrs = #[allow(clippy::op_ref)]
        => |_0| {
            assert_eq!(bt(_0, Uint::from(2_u32)) * bt(_0, Uint::from(3_u32)), bt(_0, Uint::from(6_u32)));
            assert_eq!(bt(_0, Uint::from(2_u32)) * bt(_0, Uint::ZERO), bt(_0, Uint::ZERO));

            // works for refs
            let a = bt(_0, Uint::from(11_u32));
            let b = bt(_0, Uint::from(3_u32));
            let expected = bt(_0, Uint::from(33_u32));
            assert_eq!(a * b, expected);
            assert_eq!(a * &b, expected);
            assert_eq!(&a * b, expected);
            assert_eq!(&a * &b, expected);
        }
    );

    utest!( mul_overflow_panics,
        attrs = #[should_panic(expected = "multiplication overflow")]
        => |_0| {
            let max = bt(_0, Uint::MAX);
            let _ = max * bt(_0, Uint::from(2_u64));
        }
    );

    utest!( mul_assign_works,
        attrs = #[allow(clippy::op_ref)]
        => |_0| {
            let mut a = bt(_0, Uint::from(14_u32));
            a *= bt(_0, Uint::from(2_u32));
            assert_eq!(a, bt(_0, Uint::from(28_u32)));

            // works for refs
            let mut a = bt(_0, Uint::from(10_u32));
            let b = bt(_0, Uint::from(3_u32));
            a *= &b;
            assert_eq!(a, bt(_0, Uint::from(30_u32)));
        }
    );

    utest! (pow_works,
        => |_0| {
            assert_eq!(bt(_0, Uint::from(2_u32)).checked_pow(2).unwrap(), bt(_0, Uint::from(4_u32)));
            assert_eq!(bt(_0, Uint::from(2_u32)).checked_pow(10).unwrap(), bt(_0, Uint::from(1024_u32)));

            // overflow
            let max = bt(_0, Uint::MAX);
            let result = max.checked_pow(2);
            let StdError::OverflowPow { .. } = result.unwrap_err() else {
                panic!("Expected OverflowPow error");
            };

        }
    );

    utest!( multiply_ratio,
        64 = []
        128 = []
        256 = []
        // Uint512 doesn't have multiply_ratio becase it doesn't implement NextNumber
        => |_0| {
            let base = Uint::from(500_u64);
            let min = Uint::MIN;
            let max = Uint::MAX;
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
            assert_eq!(base.checked_multiply_ratio_ceil(3_u64, 2_u64).unwrap(), Uint::from(750_u64));
            assert_eq!(base.checked_multiply_ratio_floor(3_u64, 2_u64).unwrap(), Uint::from(750_u64));
            assert_eq!(base.checked_multiply_ratio_ceil(333333_u64, 222222_u64).unwrap(), Uint::from(750_u64));
            assert_eq!(base.checked_multiply_ratio_floor(333333_u64, 222222_u64).unwrap(), Uint::from(750_u64));

            // factor 2/3
            assert_eq!(base.checked_multiply_ratio_ceil(2_u64, 3_u64).unwrap(), Uint::from(334_u64));
            assert_eq!(base.checked_multiply_ratio_floor(2_u64, 3_u64).unwrap(), Uint::from(333_u64));
            assert_eq!(base.checked_multiply_ratio_ceil(222222_u64, 333333_u64).unwrap(), Uint::from(334_u64));
            assert_eq!(base.checked_multiply_ratio_floor(222222_u64, 333333_u64).unwrap(), Uint::from(333_u64));

            // factor 5/6
            assert_eq!(base.checked_multiply_ratio_ceil(5_u64, 6_u64).unwrap(), Uint::from(417_u64));
            assert_eq!(base.checked_multiply_ratio_floor(5_u64, 6_u64).unwrap(), Uint::from(416_u64));
            assert_eq!(base.checked_multiply_ratio_ceil(100_u64, 120_u64).unwrap(), Uint::from(417_u64));
            assert_eq!(base.checked_multiply_ratio_floor(100_u64, 120_u64).unwrap(), Uint::from(416_u64));


            // 0 num works
            assert_eq!(base.checked_multiply_ratio_ceil(_0, 1_u64).unwrap(), _0);
            assert_eq!(base.checked_multiply_ratio_floor(_0, 1_u64).unwrap(), _0);

            // 1 num works
            assert_eq!(base.checked_multiply_ratio_ceil(1_u64, 1_u64).unwrap(), base);
            assert_eq!(base.checked_multiply_ratio_floor(1_u64, 1_u64).unwrap(), base);

            // not round on even divide
            let _2 = bt(_0, Uint::from(2_u64));

            assert_eq!(_2.checked_multiply_ratio_ceil(6_u64, 4_u64).unwrap(), Uint::from(3_u64));
            assert_eq!(_2.checked_multiply_ratio_floor(6_u64, 4_u64).unwrap(), Uint::from(3_u64));

        }
    );

    utest!( multiply_ratio_does_not_overflow_when_result_fits,
        64 = []
        128 = []
        256 = []
        // Uint512 doesn't have multiply_ratio becase it doesn't implement NextNumber
         => |_0| {
            // Almost max value for Uint128.
            let max = Uint::MAX;
            let reduce = Uint::from(9_u64);
            let base = max - reduce;
            dts!(_0, base, max, reduce);

            assert_eq!(base.checked_multiply_ratio_ceil(2_u64, 2_u64).unwrap(), base);
            assert_eq!(base.checked_multiply_ratio_floor(2_u64, 2_u64).unwrap(), base);
        }
    );

    utest!( multiply_ratio_overflow,
        64 = []
        128 = []
        256 = []
        // Uint512 doesn't have multiply_ratio becase it doesn't implement NextNumber
        => |_0| {
            // Almost max value for Uint128.
            let max = Uint::MAX;
            let reduce = Uint::from(9_u64);
            let base = max - reduce;
            dts!(_0, base, max, reduce);

            let result = base.checked_multiply_ratio_ceil(2_u64, 1_u64);
            let StdError::OverflowConversion { .. } = result.unwrap_err() else {
                panic!("Expected OverflowConversion error");
            };

            let result = base.checked_multiply_ratio_floor(2_u64, 1_u64);
            let StdError::OverflowConversion { .. } = result.unwrap_err() else {
                panic!("Expected OverflowConversion error");
            };
        }
    );

    utest!( multiply_ratio_divide_by_zero,
        64 = []
        128 = []
        256 = []
        // Uint512 doesn't have multiply_ratio becase it doesn't implement NextNumber
        => |_0| {
            let base = bt(_0, Uint::from(500_u64));

            let result = base.checked_multiply_ratio_ceil(1_u64, 0_u64);
            let StdError::DivisionByZero { .. } = result.unwrap_err() else {
                panic!("Expected DivisionByZero error");
            };

            let result = base.checked_multiply_ratio_floor(1_u64, 0_u64);
            let StdError::DivisionByZero { .. } = result.unwrap_err() else {
                panic!("Expected DivisionByZero error");
            };
        }
    );

    utest! (shr,
        => |_0| {
            let original = bt(_0, Uint::from(160_u64));
            assert_eq!(original >> 1, bt(_0, Uint::from(80_u64)));
            assert_eq!(original >> 3, bt(_0, Uint::from(20_u64)));
            assert_eq!(original >> 2, bt(_0, Uint::from(40_u64)));
        }
    );

    utest!( shr_overflow_panics,
        [64],
        [128],
        [256],
        [512]
        attrs = #[should_panic(expected = "shift overflow")]
        => |u, shift| {
            let original = bt(u, Uint::from(1_u64));
            let _ = original >> shift;
        }
    );

    utest! (shl,
        => |_0| {
            let original = bt(_0, Uint::from(160_u64));
            assert_eq!(original << 1, bt(_0, Uint::from(320_u64)));
            assert_eq!(original << 2, bt(_0, Uint::from(640_u64)));
            assert_eq!(original << 3, bt(_0, Uint::from(1280_u64)));
        }
    );

    utest!( shl_overflow_panics,
        [64],
        [128],
        [256],
        [512]
        attrs = #[should_panic(expected = "shift overflow")]
        => |_0, shift| {
            let original = bt(_0, Uint::from(1_u64));
            let _ = original << shift;
        }
    );

    utest!( sum,
        => |_0| {
            let nums = vec![Uint::from(17_u64), Uint::from(123_u64), Uint::from(540_u64), Uint::from(82_u64)];
            let expected = Uint::from(762_u64);

            dt(&vec![_0], &nums);
            dt(_0, expected);

            let sum_as_ref: Uint<_> = nums.iter().sum();
            assert_eq!(expected, sum_as_ref);

            let sum_as_owned = nums.into_iter().sum();
            smart_assert(expected, sum_as_owned);
        }
    );

    utest!( methods,
        => |_0| {
            let max = Uint::MAX;
            let _1 = Uint::ONE;
            let _2 = Uint::from(2_u64);
            dts!(_0, max, _1, _2);


            // checked_*
            assert!(matches!(
                max.checked_add(_1),
                Err(StdError::OverflowAdd { .. })
            ));

            assert_eq!(_1.checked_add(Uint::from(1_u64)).unwrap(), Uint::from(2_u64));
            assert!(matches!(
                _0.checked_sub(_1),
                Err(StdError::OverflowSub { .. })
            ));

            assert_eq!(Uint::from(2_u64).checked_sub(_1).unwrap(), _1);

            assert!(matches!(
                max.checked_mul(Uint::from(2_u64)),
                Err(StdError::OverflowMul { .. })
            ));

            assert_eq!(_2.checked_mul(_2).unwrap(), Uint::from(4_u64));

            assert!(matches!(
                max.checked_pow(2u32),
                Err(StdError::OverflowPow { .. })
            ));

            assert_eq!(_2.checked_pow(3).unwrap(), Uint::from(8_u64));

            assert!(matches!(
                max.checked_div(_0),
                Err(StdError::DivisionByZero { .. })
            ));

            assert_eq!(Uint::from(6_u64).checked_div(_2).unwrap(), Uint::from(3_u64));

            assert!(matches!(
                max.checked_rem(_0),
                Err(StdError::DivisionByZero { .. })
            ));

            // saturating_*

            assert_eq!(max.saturating_add(Uint::from(1_u64)), max);
            assert_eq!(_0.saturating_sub(Uint::from(1_u64)), _0);
            assert_eq!(max.saturating_mul(Uint::from(2_u64)), max);
            assert_eq!(max.saturating_pow(2), max);
        }
    );

    utest!( wrapping_methods,
        => |_0| {
            let max = Uint::MAX;
            let _1 = Uint::ONE;
            let _2 = Uint::from(2_u64);
            dts!(_0, _1, _2, max);

            // wrapping_add
            assert_eq!(_2.wrapping_add(_2), Uint::from(4_u64)); // non-wrapping
            assert_eq!(max.wrapping_add(_1), _0); // wrapping

            // wrapping_sub
            assert_eq!(Uint::from(7_u64).wrapping_sub(Uint::from(5_u64)), _2); // non-wrapping
            assert_eq!(_0.wrapping_sub(_1), max); // wrapping

            // wrapping_mul
            assert_eq!(_2.wrapping_mul(_2), Uint::from(4_u64)); // non-wrapping
            assert_eq!( max.wrapping_mul(_2), max - _1); // wrapping

            // wrapping_pow
            assert_eq!(_2.wrapping_pow(3), Uint::from(8_u64)); // non-wrapping
            assert_eq!(max.wrapping_pow(2), Uint::from(1_u64)); // wrapping
        }
    );

    utest!( saturating_methods,
        => |_0| {
            let max = Uint::MAX;
            let _1 = Uint::ONE;
            let _2 = Uint::from(2_u64);
            dts!(_0, _1, _2, max);

            // saturating_add
            assert_eq!(_2.saturating_add(_2), Uint::from(4_u64)); // non-saturating
            assert_eq!(max.saturating_add(_1), max); // saturating

            // saturating_sub
            assert_eq!(Uint::from(7_u64).saturating_sub(Uint::from(5_u64)), _2); // non-saturating
            assert_eq!(_0.saturating_sub(_1), _0); // saturating

            // saturating_mul
            assert_eq!(_2.saturating_mul(_2), Uint::from(4_u64)); // non-saturating
            assert_eq!(max.saturating_mul(_2), max); // saturating

            // saturating_pow
            assert_eq!(_2.saturating_pow(3), Uint::from(8_u64)); // non-saturating
            assert_eq!(max.saturating_pow(2), max); // saturating
        }
    );

    utest!( rem,
        attrs = #[allow(clippy::op_ref)]
        => |_0| {
            let _1 = Uint::from(1_u64);
            let _10 = Uint::from(10_u64);
            let _3 = Uint::from(3_u64);
            dts!(_0, _1, _3, _3);

            assert_eq!(_10 % Uint::from(10_u64), _0);
            assert_eq!(_10 % Uint::from(2_u64), _0);
            assert_eq!(_10 % Uint::from(1_u64), _0);
            assert_eq!(_10 % Uint::from(3_u64), Uint::from(1_u64));
            assert_eq!(_10 % Uint::from(4_u64), Uint::from(2_u64));

            assert_eq!(_10 % _3, _1);
            assert_eq!(_10 % &_3, _1);
            assert_eq!(&_10 % _3, _1);
            assert_eq!(&_10 % &_3, _1);

            // works for assign
            let mut _30 = bt(_0, Uint::from(30_u64));
            _30 %=  Uint::from(4_u64);
            assert_eq!(_30, Uint::from(2_u64));

            // works for assign refs
            let mut _25 = bt(_0, Uint::from(25_u64));
            let _6 = Uint::from(6_u64);
            _25 %= &_6;
            assert_eq!(_25, _1);

        }
    );

    utest!( rem_panics_for_zero,
        attrs = #[should_panic(expected = "division by zero")]
        => |_0| {
            let _ = Uint::from(10_u64) % _0;
        }
    );

    utest!( partial_eq,
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
                        bt(_0, Uint::from(lhs)),
                        bt(_0, Uint::from(rhs)),
                        expected
                    )
                );

            for (lhs, rhs, expected) in test_cases {
                assert_eq!(lhs == rhs, expected);
                assert_eq!(&lhs == rhs, expected);
                assert_eq!(lhs == &rhs, expected);
                assert_eq!(&lhs == &rhs, expected);
            }
        }
    );

    utest!( mul_floor,
        128 = [Udec128::new(2_u128), Udec128::from_str("0.5").unwrap(), Udec128::from_str("1.5").unwrap()]
        256 = [Udec256::new(2_u128), Udec256::from_str("0.5").unwrap(), Udec256::from_str("1.5").unwrap()]
        => |_0, _2d, _0_5d, _1_5d| {
            let _1 = Uint::from(1_u64);
            let _2 = Uint::from(2_u64);
            let _10 = Uint::from(10_u64);
            let _11 = Uint::from(11_u64);
            let max = Uint::MAX;
            dts!(_0, _1, _2, _10, _11, max);

            assert_eq!(_10.checked_mul_dec_ceil(_2d).unwrap(), Uint::from(20_u64));
            assert_eq!(_10.checked_mul_dec_floor(_2d).unwrap(), Uint::from(20_u64));

            assert_eq!(_10.checked_mul_dec_ceil(_1_5d).unwrap(), Uint::from(15_u64));
            assert_eq!(_10.checked_mul_dec_floor(_1_5d).unwrap(), Uint::from(15_u64));

            assert_eq!(_10.checked_mul_dec_ceil(_0_5d).unwrap(), Uint::from(5_u64));
            assert_eq!(_10.checked_mul_dec_floor(_0_5d).unwrap(), Uint::from(5_u64));

            // ceil works
            assert_eq!(_11.checked_mul_dec_floor(_0_5d).unwrap(), Uint::from(5_u64));
            assert_eq!(_11.checked_mul_dec_ceil(_0_5d).unwrap(), Uint::from(6_u64));

            // overflow num but not overflow result
            assert_eq!(max.checked_mul_dec_ceil(_0_5d).unwrap(), max / _2 + _1);
            assert_eq!(max.checked_mul_dec_floor(_0_5d).unwrap(), max / _2);

            // overflow num and overflow result
            assert!(matches!(
                max.checked_mul_dec_ceil(_2d),
                Err(StdError::OverflowConversion { .. })
            ));
            assert!(matches!(
                max.checked_mul_dec_floor(_2d),
                Err(StdError::OverflowConversion { .. })
            ));
        }
    );

    utest!( div_floor,
        128 = [Udec128::new(0_u128), Udec128::new(2_u128), Udec128::from_str("0.5").unwrap(), Udec128::from_str("1.5").unwrap()]
        256 = [Udec256::new(0_u128), Udec256::new(2_u128), Udec256::from_str("0.5").unwrap(), Udec256::from_str("1.5").unwrap()]
        => |_0, _0d, _2d, _0_5d, _1_5d| {
            let _1 = Uint::from(1_u64);
            let _2 = Uint::from(2_u64);
            let _10 = Uint::from(10_u64);
            let _11 = Uint::from(11_u64);
            let max = Uint::MAX;
            dts!(_0, _1, _2, _10, _11,  max);

            assert_eq!(_11.checked_div_dec_floor(_2d).unwrap(), Uint::from(5_u64));
            assert_eq!(_11.checked_div_dec_ceil(_2d).unwrap(), Uint::from(6_u64));

            assert_eq!(_10.checked_div_dec_floor(_2d).unwrap(), Uint::from(5_u64));
            assert_eq!(_10.checked_div_dec_ceil(_2d).unwrap(), Uint::from(5_u64));

            // ceil works
            assert_eq!(_11.checked_div_dec_floor(_1_5d).unwrap(), Uint::from(7_u64));
            assert_eq!(_11.checked_div_dec_ceil(_1_5d).unwrap(), Uint::from(8_u64));

            // overflow num but not overflow result
            assert_eq!(max.checked_div_dec_floor(_2d).unwrap(), max / _2);
            assert_eq!(max.checked_div_dec_ceil(_2d).unwrap(), max / _2 + _1);

            // overflow num and overflow result
            assert!(matches!(
                max.checked_div_dec_floor(_0_5d),
                Err(StdError::OverflowConversion { .. })
            ));
            assert!(matches!(
                max.checked_div_dec_ceil(_0_5d),
                Err(StdError::OverflowConversion { .. })
            ));

            // Divide by zero
            assert!(matches!(
                _10.checked_div_dec_floor(_0d),
                Err(StdError::DivisionByZero { .. })
            ));
            assert!(matches!(
                _10.checked_div_dec_ceil(_0d),
                Err(StdError::DivisionByZero { .. })
            ));
        }
    );

    utest!( multiply_fraction_by_negative,
        128 = [Dec128::from_str("-1").unwrap(), Dec128::from_str("-0").unwrap()]
        256 = [Dec256::from_str("-1").unwrap(), Dec128::from_str("-0").unwrap()]
        => |_0, n1d, n0d| {
            let lhs = bt(_0, Uint::from(123_u64));

            // Multiplying with a negative fraction should fail
            assert!(lhs.checked_mul_dec_floor(n1d).is_err());
            assert!(lhs.checked_mul_dec_ceil(n1d).is_err());
            assert!(lhs.checked_div_dec_floor(n1d).is_err());
            assert!(lhs.checked_div_dec_ceil(n1d).is_err());

            // Multiplying with negative zero is allowed though
            assert!(lhs.checked_mul_dec_floor(n0d).unwrap().is_zero());
            assert!(lhs.checked_mul_dec_ceil(n0d).unwrap().is_zero());

            // Dividing by zero should fail
            assert!(lhs.checked_div_dec_floor(n0d).is_err());
            assert!(lhs.checked_div_dec_ceil(n0d).is_err());
        }
    );
}
