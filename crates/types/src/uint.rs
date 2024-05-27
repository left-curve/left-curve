use std::{fmt::Display, str::FromStr};

use borsh::{BorshDeserialize, BorshSerialize};
use serde::{de, ser};

use crate::{
    call_inner_mapped, forward_ref_binop_typed, forward_ref_op_assign_typed, impl_assign,
    impl_base_ops, StdError, StdResult,
};

pub use traits::*;

#[derive(
    BorshSerialize, BorshDeserialize, Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord,
)]
pub struct Uint<U>(U);

impl<U> Uint<U>
where
    U: GrugNumber + PartialEq + Copy + FromStr,
{
    pub const MAX: Self = Self(U::MAX);
    pub const MIN: Self = Self(U::MIN);
    pub const ZERO: Self = Self(U::ZERO);
    pub const ONE: Self = Self(U::ONE);

    pub const fn new(value: U) -> Self {
        Self(value)
    }

    pub const fn number(self) -> U {
        self.0
    }

    pub fn is_zero(self) -> bool {
        self.0 == U::ZERO
    }
}

/// Rappresent the inner type of the [`Uint`]
///
/// This trait is used in [`generate_grug_number!`](crate::generate_grug_number!) to get the inner type of a [`Uint`]
/// and implement the conversion from the inner type to the [`Uint`]
pub trait UintInner {
    type U;
}

impl<U> UintInner for Uint<U> {
    type U = U;
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
}

impl<U> CheckedOps for Uint<U>
where
    U: CheckedOps,
{
    call_inner_mapped!(checked_add, 0);
    call_inner_mapped!(checked_sub, 0);
    call_inner_mapped!(checked_mul, 0);
    call_inner_mapped!(checked_div, 0);
    call_inner_mapped!(checked_rem, 0);
    call_inner_mapped!(checked_pow, u32);
    call_inner_mapped!(checked_shl, u32);
    call_inner_mapped!(checked_shr, u32);
}

impl_base_ops!(impl<U> Add, add for Uint<U> where sub fn checked_add);
impl_base_ops!(impl<U> Sub, sub for Uint<U> where sub fn checked_sub);
impl_base_ops!(impl<U> Mul, mul for Uint<U> where sub fn checked_mul);
impl_base_ops!(impl<U> Div, div for Uint<U> where sub fn checked_div);
impl_base_ops!(impl<U> Shl, shl for Uint<U> where sub fn checked_shl, u32);
impl_base_ops!(impl<U> Shr, shr for Uint<U> where sub fn checked_shr, u32);

impl_assign!(impl<U> AddAssign, add_assign for Uint<U> where sub fn checked_add);
impl_assign!(impl<U> SubAssign, sub_assign for Uint<U> where sub fn checked_sub);
impl_assign!(impl<U> MulAssign, mul_assign for Uint<U> where sub fn checked_mul);
impl_assign!(impl<U> DivAssign, div_assign for Uint<U> where sub fn checked_div);
impl_assign!(impl<U> ShrAssign, shr_assign for Uint<U> where sub fn checked_shr, u32);
impl_assign!(impl<U> ShlAssign, shl_assign for Uint<U> where sub fn checked_shl, u32);

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

// TODO: Is worth create macros to impl below traits?

impl<U> FromStr for Uint<U>
where
    U: FromStr,
    <U as FromStr>::Err: ToString,
{
    type Err = StdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        U::from_str(s).map(Self).map_err(|err| StdError::parse_number::<Self>(s, err))
    }
}

impl<U> From<Uint<U>> for String
where
    U: std::fmt::Display,
{
    fn from(value: Uint<U>) -> Self {
        value.to_string()
    }
}

impl<U> std::fmt::Display for Uint<U>
where
    U: std::fmt::Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0.to_string())
    }
}

impl<U> ser::Serialize for Uint<U>
where
    U: std::fmt::Display,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<'de, U> de::Deserialize<'de> for Uint<U>
where
    U: Default + FromStr,
    <U as FromStr>::Err: Display,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_str(UintVisitor::<U>::default())
    }
}

#[derive(Default)]
struct UintVisitor<U> {
    _marker: std::marker::PhantomData<U>,
}

impl<'de, U> de::Visitor<'de> for UintVisitor<U>
where
    U: FromStr,
    <U as FromStr>::Err: Display,
{
    type Value = Uint<U>;

    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        // TODO: Change this message in base at the type of U
        f.write_str("a string-encoded 256-bit unsigned integer")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        v.parse::<U>().map(Uint::<U>).map_err(E::custom)
    }
}

// TODO: Move this to a separate file (could be usefull for other types)
mod traits {
    use crate::StdResult;

    pub trait GrugNumber {
        const MAX: Self;
        const MIN: Self;
        const ZERO: Self;
        const ONE: Self;
    }

    pub trait Bytable<const S: usize> {
        fn from_be_bytes(data: [u8; S]) -> Self;
        fn from_le_bytes(data: [u8; S]) -> Self;
        fn to_be_bytes(self) -> [u8; S];
        fn to_le_bytes(self) -> [u8; S];
        fn byte_len() -> usize {
            S
        }
    }

    pub trait CheckedOps: Sized {
        fn checked_add(self, other: Self) -> StdResult<Self>;
        fn checked_sub(self, other: Self) -> StdResult<Self>;
        fn checked_mul(self, other: Self) -> StdResult<Self>;
        fn checked_div(self, other: Self) -> StdResult<Self>;
        fn checked_rem(self, other: Self) -> StdResult<Self>;
        fn checked_pow(self, other: u32) -> StdResult<Self>;
        fn checked_shl(self, other: u32) -> StdResult<Self>;
        fn checked_shr(self, other: u32) -> StdResult<Self>;
    }
}

// TODO: Move this to a separate file (could be usefull for other types)
mod macros {

    /// Generate a [`Unit`](super::Uint) type for a given inner type.
    ///
    /// ### Example:
    /// ```ignore
    /// generate_grug_number!(
    ///     // The name of the Uint
    ///     name = Uint128g,     
    ///     // Inner type of the Uint
    ///     inner_type = u128,   
    ///     // Minimum value of the Uint
    ///     min = u128::MIN,     
    ///     // Maximum value of the Uint
    ///     max = u128::MAX,     
    ///     // Zero value of the Uint
    ///     zero = 0,           
    ///     // One value of the Uint
    ///     one = 1,            
    ///     // Byte length of the Uint
    ///     byte_len = 16,    
    ///     // (Optional)
    ///     // If std, impl_bytable_std be will call
    ///     // If bnum, impl_bytable_bnum be will call
    ///     // If skipped, no impl_bytable will be called.
    ///     // This require to implement the Bytable trait manually
    ///     impl_bytable = std,
    ///     // Implement From | TryInto from other Uint types
    ///     from_uint = [Uint64g]
    /// );
    #[macro_export]
    macro_rules! generate_grug_number {
        (
            name = $name:ident,
            inner_type = $inner:ty,
            min = $min:expr,
            max = $max:expr,
            zero = $zero:expr,
            one = $one:expr,
            byte_len = $byte_len:literal,
            impl_bytable = std,
            from_uint = [$($from_uint:ty),*]

        ) => {
            impl_bytable_std!($inner, $byte_len);
            generate_grug_number!(
                name = $name,
                inner_type = $inner,
                min = $min,
                max = $max,
                zero = $zero,
                one = $one,
                byte_len = $byte_len,
                from_uint = [$($from_uint),*]
            );
        };
        (
            name = $name:ident,
            inner_type = $inner:ty,
            min = $min:expr,
            max = $max:expr,
            zero = $zero:expr,
            one = $one:expr,
            byte_len = $byte_len:literal,
            impl_bytable = bnum,
            from_uint = [$($from_uint:ty),*]

        ) => {
            impl_bytable_bnum!($inner, $byte_len);
            generate_grug_number!(
                name = $name,
                inner_type = $inner,
                min = $min,
                max = $max,
                zero = $zero,
                one = $one,
                byte_len = $byte_len,
                from_uint = [$($from_uint),*]
            );
        };
        (
            name = $name:ident,
            inner_type = $inner:ty,
            min = $min:expr,
            max = $max:expr,
            zero = $zero:expr,
            one = $one:expr,
            byte_len = $byte_len:literal,
            from_uint = [$($from_uint:ty),*]

        ) => {
            impl_number_bound!($inner, $max, $min, $zero, $one);
            impl_checked_ops!($inner);
            pub type $name = Uint<$inner>;

            // Impl From Uint and from inner type
            $(
                impl From<$from_uint> for $name {
                    fn from(value: $from_uint) -> Self {
                        let others_byte = value.to_le_bytes();
                        let mut bytes: [u8; $byte_len] = [0; $byte_len];
                        for i in 0..others_byte.len() {
                            bytes[i] = others_byte[i];
                        }
                        Self::from_le_bytes(bytes)
                    }
                }

                impl From<<$from_uint as UintInner>::U> for $name {
                    fn from(value: <$from_uint as UintInner>::U) -> Self {
                        let others_byte = value.to_le_bytes();
                        let mut bytes: [u8; $byte_len] = [0; $byte_len];
                        for i in 0..others_byte.len() {
                            bytes[i] = others_byte[i];
                        }
                        Self::from_le_bytes(bytes)
                    }
                }

                impl TryInto<$from_uint> for $name {
                    type Error = StdError;

                    fn try_into(self) -> StdResult<$from_uint> {
                        let other_b_l = <$from_uint>::byte_len();
                        let bytes = self.to_le_bytes();
                        let (lower, higher) = bytes.split_at(other_b_l);
                        if higher.iter().any(|&b| b != 0) {
                            // TODO: Change this error after implementing FromStr for Uint
                            return Err(StdError::Generic("Conversion error!".to_string()));
                        }
                        Ok(<$from_uint>::from_le_bytes(lower.try_into()?))
                    }

                }

                // TODO: Maybe generate a closure to avoid code duplication
                impl TryInto<<$from_uint as UintInner>::U> for $name {
                    type Error = StdError;

                    fn try_into(self) -> StdResult<<$from_uint as UintInner>::U> {
                        let other_b_l = <$from_uint>::byte_len();
                        let bytes = self.to_le_bytes();
                        let (lower, higher) = bytes.split_at(other_b_l);
                        if higher.iter().any(|&b| b != 0) {
                            // TODO: Change this error after implementing FromStr for Uint
                            return Err(StdError::Generic("Conversion error!".to_string()));
                        }
                        Ok(<$from_uint>::from_le_bytes(lower.try_into()?).into())
                    }

                }
            )*

            impl From<$inner> for $name {
                fn from(value: $inner) -> Self {
                    Self::new(value)
                }
            }

            impl From<$name> for $inner {
                fn from(value: $name) -> Self {
                   value.number()
                }
            }

        };
    }

    /// **Syntax**:
    ///
    ///  `impl_grug_number!(type, max, min, zero, one)`
    ///
    /// **Example**:
    ///  ```ignore
    /// impl_grug_number!(u64, u64::MAX, u64::MIN, 0, 1)
    /// ```
    #[macro_export]
    macro_rules! impl_number_bound {
        ($t:ty, $max:expr, $min:expr, $zero:expr, $one:expr) => {
            impl GrugNumber for $t {
                const MAX: Self = $max;
                const MIN: Self = $min;
                const ZERO: Self = $zero;
                const ONE: Self = $one;
            }

            // This is a compile-time check to ensure that the constants are of the correct type.
            const _: () = {
                const fn _check_type(_: $t) {}
                _check_type($max);
                _check_type($min);
                _check_type($zero);
                _check_type($one);
            };
        };
    }

    /// Implements [`Bytable`](super::Bytable) for std types (u64, u128, ...)
    #[macro_export]
    macro_rules! impl_bytable_std {
        ($t:ty, $rot:literal) => {
            #[deny(unconditional_recursion)]
            impl Bytable<$rot> for $t {
                fn from_be_bytes(data: [u8; $rot]) -> Self {
                    Self::from_be_bytes(data)
                }

                fn from_le_bytes(data: [u8; $rot]) -> Self {
                    Self::from_le_bytes(data)
                }

                fn to_be_bytes(self) -> [u8; $rot] {
                    self.to_be_bytes()
                }

                fn to_le_bytes(self) -> [u8; $rot] {
                    self.to_le_bytes()
                }
            }
        };
    }

    /// Implements [`Bytable`](super::Bytable) for [`bnum`] types (U256, U512, ...)
    #[macro_export]
    macro_rules! impl_bytable_bnum {
        ($t:ty, $rot:literal) => {
            impl Bytable<$rot> for $t {
                fn from_be_bytes(data: [u8; $rot]) -> Self {
                    let mut bytes = [0u64; $rot / 8];
                    for i in 0..$rot / 8 {
                        bytes[i] = u64::from_le_bytes([
                            data[($rot / 8 - i - 1) * 8 + 7],
                            data[($rot / 8 - i - 1) * 8 + 6],
                            data[($rot / 8 - i - 1) * 8 + 5],
                            data[($rot / 8 - i - 1) * 8 + 4],
                            data[($rot / 8 - i - 1) * 8 + 3],
                            data[($rot / 8 - i - 1) * 8 + 2],
                            data[($rot / 8 - i - 1) * 8 + 1],
                            data[($rot / 8 - i - 1) * 8],
                        ])
                    }
                    Self::from_digits(bytes)
                }

                fn from_le_bytes(data: [u8; $rot]) -> Self {
                    let mut bytes = [0u64; $rot / 8];
                    for i in 0..$rot / 8 {
                        bytes[i] = u64::from_le_bytes([
                            data[i * 8],
                            data[i * 8 + 1],
                            data[i * 8 + 2],
                            data[i * 8 + 3],
                            data[i * 8 + 4],
                            data[i * 8 + 5],
                            data[i * 8 + 6],
                            data[i * 8 + 7],
                        ])
                    }
                    Self::from_digits(bytes)
                }

                fn to_be_bytes(self) -> [u8; $rot] {
                    let words = self.digits();
                    let mut bytes: [[u8; 8]; $rot / 8] = [[0u8; 8]; $rot / 8];
                    for i in 0..$rot / 8 {
                        bytes[i] = words[$rot / 8 - i - 1].to_be_bytes();
                    }

                    unsafe { std::mem::transmute(bytes) }
                }

                fn to_le_bytes(self) -> [u8; $rot] {
                    let words = self.digits();
                    let mut bytes: [[u8; 8]; $rot / 8] = [[0u8; 8]; $rot / 8];
                    for i in 0..$rot / 8 {
                        bytes[i] = words[i].to_le_bytes();
                    }

                    unsafe { std::mem::transmute(bytes) }
                }
            }
        };
    }

    #[macro_export]
    macro_rules! impl_checked_ops {
        ($t:ty) => {
            impl CheckedOps for $t {
                fn checked_add(self, other: Self) -> StdResult<Self> {
                    self.checked_add(other).ok_or_else(|| StdError::overflow_add(self, other))
                }

                fn checked_sub(self, other: Self) -> StdResult<Self> {
                    self.checked_sub(other).ok_or_else(|| StdError::overflow_sub(self, other))
                }

                fn checked_mul(self, other: Self) -> StdResult<Self> {
                    self.checked_mul(other).ok_or_else(|| StdError::overflow_mul(self, other))
                }

                fn checked_div(self, other: Self) -> StdResult<Self> {
                    self.checked_div(other).ok_or_else(|| StdError::division_by_zero(self))
                }

                fn checked_rem(self, other: Self) -> StdResult<Self> {
                    self.checked_rem(other).ok_or_else(|| StdError::division_by_zero(self))
                }

                fn checked_pow(self, other: u32) -> StdResult<Self> {
                    self.checked_pow(other).ok_or_else(|| StdError::overflow_pow(self, other))
                }

                fn checked_shl(self, other: u32) -> StdResult<Self> {
                    self.checked_shl(other).ok_or_else(|| StdError::overflow_shl(self, other))
                }

                fn checked_shr(self, other: u32) -> StdResult<Self> {
                    self.checked_shr(other).ok_or_else(|| StdError::overflow_shr(self, other))
                }
            }
        };
    }

    #[macro_export]
    macro_rules! impl_base_ops {
        (impl<$($gen:tt),*> $imp:ident, $method:ident for $t:ty where sub fn $sub_method:ident) =>

        {
            impl<$($gen),*> std::ops::$imp for $t
            where
                $t: CheckedOps
            {
                type Output = Self;

                fn $method(self, other: Self) -> Self {
                    self.$sub_method(other).unwrap_or_else(|err| panic!("{err}"))
                }
            }
        };
        (impl<$($gen:tt),*> $imp:ident, $method:ident for $t:ty where sub fn $sub_method:ident, $other:ty) => {
            impl<$($gen),*> std::ops::$imp<$other> for $t
            where
                $t: CheckedOps
            {
                type Output = Self;

                fn $method(self, other: $other) -> Self {
                    self.$sub_method(other).unwrap_or_else(|err| panic!("{err}"))
                }
            }
        };
    }

    #[macro_export]
    macro_rules! impl_assign {
        (impl<$($gen:tt),*> $imp:ident, $method:ident for $t:ty where sub fn $sub_method:ident) => {
            impl<$($gen),*>core::ops::$imp for $t
            where
                $t: CheckedOps + Copy
            {
                fn $method(&mut self, other: Self) {
                    *self = (*self).$sub_method(other).unwrap_or_else(|err| panic!("{err}"))
                }
            }
        };
        (impl<$($gen:tt),*> $imp:ident, $method:ident for $t:ty where sub fn $sub_method:ident, $other:ty) => {
            impl<U> core::ops::$imp<$other> for $t
            where
            $t: CheckedOps + Copy            {
                fn $method(&mut self, other: $other) {
                    *self = (*self).$sub_method(other).unwrap_or_else(|err| panic!("{err}"))
                }
            }
        };
    }

    #[macro_export]
    macro_rules! call_inner_mapped {
        ($op:ident, $other:ident) => {
            fn $op(self, other: $other) -> StdResult<Self> {
                self.0.$op(other).map(|val| Self(val))
            }
        };
        ($op:ident, $inner:tt) => {
            fn $op(self, other: Self) -> StdResult<Self> {
                self.0.$op(other.$inner).map(|val| Self(val))
            }
        };
    }

    #[macro_export]
    macro_rules! forward_ref_binop_typed {
        (impl<$($gen:tt),*> $imp:ident, $method:ident for $t:ty, $u:ty) => {

            impl<$($gen),*> std::ops::$imp<$u> for &'_ $t where $t: std::ops::$imp<$u> + Copy {
                type Output = <$t as std::ops::$imp<$u>>::Output;

                #[inline]
                fn $method(self, other: $u) -> <$t as std::ops::$imp<$u>>::Output {
                    std::ops::$imp::$method(*self, other)
                }
            }

            impl<$($gen),*> std::ops::$imp<&$u> for $t where $t: std::ops::$imp<$u> + Copy {
                type Output = <$t as std::ops::$imp<$u>>::Output;

                #[inline]
                fn $method(self, other: &$u) -> <$t as std::ops::$imp<$u>>::Output {
                    std::ops::$imp::$method(self, *other)
                }
            }

            impl<$($gen),*> std::ops::$imp<&$u> for &'_ $t where $t: std::ops::$imp<$u> + Copy {
                type Output = <$t as std::ops::$imp<$u>>::Output;

                #[inline]
                fn $method(self, other: &$u) -> <$t as std::ops::$imp<$u>>::Output {
                    std::ops::$imp::$method(*self, *other)
                }
            }
        };
    }

    #[macro_export]
    macro_rules! forward_ref_op_assign_typed {
        (impl<$($gen:tt),*> $imp:ident, $method:ident for $t:ty, $u:ty) => {

            impl <$($gen),*> std::ops::$imp<&$u> for $t where $t: std::ops::$imp<$u> + Copy {
                #[inline]
                fn $method(&mut self, other: &$u) {
                    std::ops::$imp::$method(self, *other);
                }
            }
        };
    }
}
