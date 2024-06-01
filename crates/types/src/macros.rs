//! This file contains macros that are mainly used to define math types, namely
//! `Uint`, `Int`, `Decimal`, and `SignedDecimal`. They are generally not
//! intended for use outside of this crate.

/// Generate a [`Unit`](super::Int) type for a given inner type.
///
/// ### Example
///
/// ```ignore
/// generate_int!(
///     // The name of the Int
///     name = Int128,
///     // Inner type of the Int
///     inner_type = i128,
///     // Implement From | TryInto from other Int types
///     // Safe type where overflow is not possible
///     // It also impls Base ops (Add, Sub ecc..) vs this type
///     from_int = [Int64, Uint64]
///     // Implement From | TryInto from other std types
///     // Safe type where overflow is not possible
///     // It also impls Base ops (Add, Sub ecc..) vs this type
///     from_std = [u32, u16, u8, i32, i16, i8]
///     // Implement TryFrom | TryInto from other Int types
///     // Unsafe type where overflow is possible
///     try_from_int = [Uint128]
/// );
#[macro_export]
macro_rules! generate_int {
    (
        name = $name:ident,
        inner_type = $inner:ty,
        from_int = [$($from:ty),*],
        from_std = [$($from_std:ty),*],
        try_from_int = [$($try_from:ty),*]
    ) => {
        pub type $name = Int<$inner>;

        // --- Impl From Int and from inner type ---
        $(
            // Ex: From<Uint64> for Uint128
            impl From<$from> for $name {
                fn from(value: $from) -> Self {
                    Self::from_str(&value.to_string()).unwrap() // Safe unwrap
                }
            }

            // Ex: From<u64> for Uint128
            impl From<<$from as Inner>::U> for $name {
                fn from(value: <$from as Inner>::U) -> Self {
                    Self::from_str(&value.to_string()).unwrap() // Safe unwrap
                }
            }

            // Ex: TryInto<Uint64> for Uint128
            impl TryInto<$from> for $name {
                type Error = StdError;
                fn try_into(self) -> StdResult<$from> {
                    <$from>::from_str(&self.to_string())
                }

            }

            // Ex: TryInto<u64> for Uint128
            impl TryInto<<$from as Inner>::U> for $name {
                type Error = StdError;
                fn try_into(self) -> StdResult<<$from as Inner>::U> {
                    <$from>::from_str(&self.to_string()).map(Into::into)
                }
            }

            impl_all_ops_and_assign!($name, $from);
        )*

        // --- Impl From std ---
        $(
            // Ex: From<u32> for Uint128
            impl From<$from_std> for $name {
                fn from(value: $from_std) -> Self {
                    Self::new_from(value)
                }
            }

            impl TryInto<$from_std> for $name {
                type Error = StdError;
                fn try_into(self) -> StdResult<$from_std> {
                    <$from_std>::from_str(&self.to_string())
                        .map_err(|_| StdError::overflow_conversion::<_, $from_std>(self))
                }
            }

            // --- Impl ops ---
            impl_all_ops_and_assign!($name, $from_std);
        )*

        $(
            // Ex: TryFrom<Uint128> for Int128
            impl TryFrom<$try_from> for $name {
                type Error = StdError;
                fn try_from(value: $try_from) -> StdResult<Self> {
                    Self::from_str(&value.to_string())
                }
            }

            // Ex: From<u64> for Uint128
            impl TryFrom<<$try_from as Inner>::U> for $name {
                type Error = StdError;
                fn try_from(value: <$try_from as Inner>::U) -> StdResult<Self> {
                    Self::from_str(&value.to_string())
                }
            }

            // Ex: TryInto<Uint64> for Uint128
            impl TryInto<$try_from> for $name {
                type Error = StdError;
                fn try_into(self) -> StdResult<$try_from> {
                    <$try_from>::from_str(&self.to_string())
                }

            }

            // Ex: TryInto<u64> for Uint128
            impl TryInto<<$try_from as Inner>::U> for $name {
                type Error = StdError;
                fn try_into(self) -> StdResult<<$try_from as Inner>::U> {
                    <$try_from>::from_str(&self.to_string()).map(Into::into)
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

/// Generate a [`Decimal`](super::Decimal) type for a given inner type.
///
/// ### Example
///
/// ```ignore
/// generate_int!(
///     // The name of the Int
///     name = SignedDecimal256,
///     // Inner type of the Int
///     inner_type = I256,
///     // Number of decimal places
///     decimal_places = 18,
///     // Implement From | TryInto from other Decimal types
///     // Safe type where overflow is not possible
///     // It also impls Base ops (Add, Sub ecc..) vs this type
///     from_dec = [SignedDecimal128, Decimal128]
///     // Implement TryFrom | TryInto from other Int types
///     // Unsafe type where overflow is possible
///     try_from_dec = [Decimal256]
/// );
#[macro_export]
macro_rules! generate_decimal {
    (
        name = $name:ident,
        inner_type = $inner:ty,
        decimal_places = $decimal_places:expr,
        from_dec = [$($from:ty),*],
        try_from_dec = [$($try_from:ty),*]
    ) => {
        pub type $name = Decimal<$inner, $decimal_places>;

        impl $name {
            pub const DECIMAL_PLACES: usize = $decimal_places;
        }

        $(
            // Ex: From<Decimal128> for Decimal256
            impl From<$from> for $name {
                fn from(value: $from) -> Self {
                    // This is safe.
                    // But it's depend on the data passed on the macro
                    Self::from_str(&value.to_string()).unwrap()
                }
            }

            // Ex: TryInto<Decimal128> for Decimal256
            impl TryInto<$from> for $name {
                type Error = StdError;
                fn try_into(self) -> StdResult<$from> {
                    <$from>::from_str(&self.to_string())
                }
            }

            impl_all_ops_and_assign!($name, $from);
        )*

        $(
            // Ex: TryFrom<Decimal128> for Decimal256
            impl TryFrom<$try_from> for $name {
                type Error = StdError;
                fn try_from(value: $try_from) -> StdResult<Self> {
                    // This is safe.
                    // But it's depend on the data passed on the macro
                    Self::from_str(&value.to_string())
                }
            }

            // Ex: TryInto<Decimal128> for Decimal256
            impl TryInto<$try_from> for $name {
                type Error = StdError;
                fn try_into(self) -> StdResult<$try_from> {
                    <$try_from>::from_str(&self.to_string())
                }
            }
        )*
    };
}

/// **Syntax**:
///
/// ```ignore
/// impl_number_bound!(type, max, min, zero, one);
/// ```
///
/// **Example**:
///
/// ```ignore
/// impl_number_bound!(u64, u64::MAX, u64::MIN, 0, 1);
/// ```
#[macro_export]
macro_rules! impl_number_bound {
    ($t:ty, $max:expr, $min:expr, $zero:expr, $one:expr, $ten:expr) => {
        impl NumberConst for $t {
            const MAX: Self = $max;
            const MIN: Self = $min;
            const ONE: Self = $one;
            const TEN: Self = $ten;
            const ZERO: Self = $zero;
        }

        // This is a compile-time check to ensure that the constants are of the
        // correct types.
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

            fn grow_be_bytes<const INPUT_SIZE: usize>(data: [u8; INPUT_SIZE]) -> [u8; $rot] {
                grow_be_uint::<INPUT_SIZE, $rot>(data)
            }

            fn grow_le_bytes<const INPUT_SIZE: usize>(data: [u8; INPUT_SIZE]) -> [u8; $rot] {
                grow_le_uint::<INPUT_SIZE, $rot>(data)
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

            fn grow_be_bytes<const INPUT_SIZE: usize>(data: [u8; INPUT_SIZE]) -> [u8; $rot] {
                grow_be_uint::<INPUT_SIZE, $rot>(data)
            }

            fn grow_le_bytes<const INPUT_SIZE: usize>(data: [u8; INPUT_SIZE]) -> [u8; $rot] {
                grow_le_uint::<INPUT_SIZE, $rot>(data)
            }
        }
    };
}

#[macro_export]
macro_rules! impl_bytable_ibnum {
    ($t:ty, $rot:literal, $unsigned:ty) => {
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
                Self::from_bits(<$unsigned>::from_digits(bytes))
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
                Self::from_bits(<$unsigned>::from_digits(bytes))
            }

            fn to_be_bytes(self) -> [u8; $rot] {
                let words = self.to_bits();
                let words = words.digits();
                let mut bytes: [[u8; 8]; $rot / 8] = [[0u8; 8]; $rot / 8];
                for i in 0..$rot / 8 {
                    bytes[i] = words[$rot / 8 - i - 1].to_be_bytes();
                }

                unsafe { std::mem::transmute(bytes) }
            }

            fn to_le_bytes(self) -> [u8; $rot] {
                let words = self.to_bits();
                let words = words.digits();
                let mut bytes: [[u8; 8]; $rot / 8] = [[0u8; 8]; $rot / 8];
                for i in 0..$rot / 8 {
                    bytes[i] = words[i].to_le_bytes();
                }

                unsafe { std::mem::transmute(bytes) }
            }

            fn grow_be_bytes<const INPUT_SIZE: usize>(data: [u8; INPUT_SIZE]) -> [u8; $rot] {
                grow_be_int::<INPUT_SIZE, $rot>(data)
            }

            fn grow_le_bytes<const INPUT_SIZE: usize>(data: [u8; INPUT_SIZE]) -> [u8; $rot] {
                grow_le_int::<INPUT_SIZE, $rot>(data)
            }
        }
    };
}

#[macro_export]
macro_rules! impl_checked_ops {
    ($t:ty) => {
        fn checked_add(self, other: Self) -> StdResult<Self> {
            self.checked_add(other)
                .ok_or_else(|| StdError::overflow_add(self, other))
        }

        fn checked_sub(self, other: Self) -> StdResult<Self> {
            self.checked_sub(other)
                .ok_or_else(|| StdError::overflow_sub(self, other))
        }

        fn checked_mul(self, other: Self) -> StdResult<Self> {
            self.checked_mul(other)
                .ok_or_else(|| StdError::overflow_mul(self, other))
        }

        fn checked_div(self, other: Self) -> StdResult<Self> {
            self.checked_div(other)
                .ok_or_else(|| StdError::division_by_zero(self))
        }

        fn checked_rem(self, other: Self) -> StdResult<Self> {
            self.checked_rem(other)
                .ok_or_else(|| StdError::division_by_zero(self))
        }

        fn checked_pow(self, other: u32) -> StdResult<Self> {
            self.checked_pow(other)
                .ok_or_else(|| StdError::overflow_pow(self, other))
        }

        fn checked_shl(self, other: u32) -> StdResult<Self> {
            self.checked_shl(other)
                .ok_or_else(|| StdError::overflow_shl(self, other))
        }

        fn checked_shr(self, other: u32) -> StdResult<Self> {
            self.checked_shr(other)
                .ok_or_else(|| StdError::overflow_shr(self, other))
        }

        fn checked_ilog2(self) -> StdResult<u32> {
            self.checked_ilog2().ok_or_else(|| StdError::zero_log())
        }

        fn checked_ilog10(self) -> StdResult<u32> {
            self.checked_ilog10().ok_or_else(|| StdError::zero_log())
        }

        fn wrapping_add(self, other: Self) -> Self {
            self.wrapping_add(other)
        }

        fn wrapping_sub(self, other: Self) -> Self {
            self.wrapping_sub(other)
        }

        fn wrapping_mul(self, other: Self) -> Self {
            self.wrapping_mul(other)
        }

        fn wrapping_pow(self, other: u32) -> Self {
            self.wrapping_pow(other)
        }

        fn saturating_add(self, other: Self) -> Self {
            self.saturating_add(other)
        }

        fn saturating_sub(self, other: Self) -> Self {
            self.saturating_sub(other)
        }

        fn saturating_mul(self, other: Self) -> Self {
            self.saturating_mul(other)
        }

        fn saturating_pow(self, other: u32) -> Self {
            self.saturating_pow(other)
        }

        fn is_zero(self) -> bool {
            self == Self::ZERO
        }
    };
}

#[macro_export]
macro_rules! impl_checked_ops_unsigned {
    ($t:ty) => {
        impl CheckedOps for $t {
            impl_checked_ops!($t);

            fn abs(self) -> Self {
                self
            }
        }
    };
}

#[macro_export]
macro_rules! impl_checked_ops_signed {
    ($t:ty) => {
        impl CheckedOps for $t {
            impl_checked_ops!($t);

            fn abs(self) -> Self {
                self.abs()
            }
        }
    };
}

#[macro_export]
macro_rules! impl_next {
    ($t:ty, $next:ty) => {
        impl NextNumber for $t {
            type Next = $next;
        }
    };
}

#[macro_export]
macro_rules! impl_all_ops_and_assign {
    ($name:ident, $other:ty) => {
        impl_base_ops!(impl Add, add for $name as $other where sub fn checked_add);
        impl_base_ops!(impl Sub, sub for $name as $other where sub fn checked_sub);
        impl_base_ops!(impl Mul, mul for $name as $other where sub fn checked_mul);
        impl_base_ops!(impl Div, div for $name as $other where sub fn checked_div);

        forward_ref_binop!(impl Add, add for $name, $other);
        forward_ref_binop!(impl Sub, sub for $name, $other);
        forward_ref_binop!(impl Mul, mul for $name, $other);
        forward_ref_binop!(impl Div, div for $name, $other);

        forward_ref_binop!(impl Add, add for $other, $name);
        forward_ref_binop!(impl Sub, sub for $other, $name);
        forward_ref_binop!(impl Mul, mul for $other, $name);
        forward_ref_binop!(impl Div, div for $other, $name);

        impl_assign!(impl AddAssign, add_assign for $name as $other where sub fn checked_add);
        impl_assign!(impl SubAssign, sub_assign for $name as $other where sub fn checked_sub);
        impl_assign!(impl MulAssign, mul_assign for $name as $other where sub fn checked_mul);
        impl_assign!(impl DivAssign, div_assign for $name as $other where sub fn checked_div);

        forward_ref_op_assign!(impl AddAssign, add_assign for $name, $other);
        forward_ref_op_assign!(impl SubAssign, sub_assign for $name, $other);
        forward_ref_op_assign!(impl MulAssign, mul_assign for $name, $other);
        forward_ref_op_assign!(impl DivAssign, div_assign for $name, $other);
    };
}

#[macro_export]
macro_rules! impl_base_ops {
    // args type = Self
    (impl<$($gen:tt),*> $imp:ident, $method:ident for $t:ty where sub fn $sub_method:ident) => {
        impl<$($gen),*> std::ops::$imp for $t
        where
            $t: CheckedOps,
        {
            type Output = Self;

            fn $method(self, other: Self) -> Self {
                self.$sub_method(other).unwrap_or_else(|err| panic!("{err}"))
            }
        }
    };

    // args type = other
    (impl<$($gen:tt),*> $imp:ident, $method:ident for $t:ty where sub fn $sub_method:ident, $other:ty) => {
        impl<$($gen),*> std::ops::$imp<$other> for $t
        where
            $t: CheckedOps,
        {
            type Output = Self;

            fn $method(self, other: $other) -> Self {
                self.$sub_method(other).unwrap_or_else(|err| panic!("{err}"))
            }
        }
    };

    // Ops self for other, output = Self
    // Ex: Add<Uint64> for Uint128 => Uint128
    // Ex: Add<Decimal128> for Decimal256 => Decimal256
    (impl $imp:ident, $method:ident for $t:ty as $other:ty where sub fn $sub_method:ident) => {
        impl std::ops::$imp<$other> for $t {
            type Output = Self;
            fn $method(self, other: $other) -> Self {
                self.$sub_method(other.into()).unwrap_or_else(|err| panic!("{err}"))
            }
        }

        impl std::ops::$imp<$t> for $other {
            type Output = $t;

            fn $method(self, other: $t) -> $t {
                other + self
            }
        }
    };

    // Decimal Self
    (impl Decimal with $imp:ident, $method:ident for $t:ty where sub fn $sub_method:ident) => {
        impl<U, const S: usize> std::ops::$imp for $t
        where
            Int<U>: NextNumber + CheckedOps,
            <Int<U> as NextNumber>::Next: From<Int<U>> + TryInto<Int<U>> + CheckedOps + ToString + Clone,
            U: NumberConst + Clone + PartialEq + Copy + FromStr,
        {
            type Output = Self;

            fn $method(self, other: Self) -> Self {
                self.$sub_method(other).unwrap_or_else(|err| panic!("{err}"))
            }
        }
    };

}

#[macro_export]
macro_rules! impl_signed_ops {
    // Not
    (impl<$($gen:tt),*> Not for $t:ty) => {
        impl<$($gen),*> std::ops::Not for $t
        where
            U: Not + Not<Output = U>,
        {
            type Output = Self;

            fn not(self) -> Self {
                Self(!self.0)
            }
        }
    };

    // Neg
    (impl<$($gen:tt),*> Neg for $t:ty) => {
        impl<$($gen),*> std::ops::Neg for $t
        where
            U: Neg + Neg<Output = U>,
        {
            type Output = Self;

            fn neg(self) -> Self {
                Self(-self.0)
            }
        }
    };

    // Not Decimal
    (impl Not for $t:ident) => {
        impl<U, const S: usize> std::ops::Not for $t<U,S>
        where
            U: Not + Not<Output = U>,
        {
            type Output = Self;

            fn not(self) -> Self {
                Self(!self.0)
            }
        }
    };

    // Neg Decimal
    (impl Neg for $t:ident) => {
        impl<U, const S: usize> std::ops::Neg for $t<U,S>
        where
            U: Neg + Neg<Output = U>,
        {
            type Output = Self;

            fn neg(self) -> Self {
                Self(-self.0)
            }
        }

    };
}

#[macro_export]
macro_rules! impl_assign {
        // args type = Self
        (impl<$($gen:tt),*> $imp:ident, $method:ident for $t:ty where sub fn $sub_method:ident) => {
            impl<$($gen),*>core::ops::$imp for $t
            where
                $t: CheckedOps + Copy,
            {
                fn $method(&mut self, other: Self) {
                    *self = (*self).$sub_method(other).unwrap_or_else(|err| panic!("{err}"))
                }
            }
        };

        // args type = other
        (impl<$($gen:tt),*> $imp:ident, $method:ident for $t:ty where sub fn $sub_method:ident, $other:ty) => {
            impl<U> core::ops::$imp<$other> for $t
            where
                $t: CheckedOps + Copy,
            {
                fn $method(&mut self, other: $other) {
                    *self = (*self).$sub_method(other).unwrap_or_else(|err| panic!("{err}"))
                }
            }
        };

        // Decimal
        (impl Decimal with $imp:ident, $method:ident for $t:ty where sub fn $sub_method:ident) =>
        {
            impl<U, const S: usize> std::ops::$imp for $t
            where
                Int<U>: NextNumber + CheckedOps,
                <Int<U> as NextNumber>::Next: From<Int<U>> + TryInto<Int<U>> + CheckedOps + ToString + Clone,
                U: NumberConst + Clone + PartialEq + Copy + FromStr,
            {
                fn $method(&mut self, other: Self) {
                    *self = (*self).$sub_method(other).unwrap_or_else(|err| panic!("{err}"))
                }
            }
        };

        // Ops self for other, output = Self
        // Ex: AddAssign<Uint64> for Uint128;
        (impl $imp:ident, $method:ident for $t:ty as $other:ty where sub fn $sub_method:ident) => {
            impl std::ops::$imp<$other> for $t {
                fn $method(&mut self, other: $other) {
                    *self = (*self).$sub_method(other.into()).unwrap_or_else(|err| panic!("{err}"))
                }
            }
        };
    }

#[macro_export]
macro_rules! call_inner {
    (fn $op:ident,arg $other:ident, => Result<Self>) => {
        fn $op(self, other: $other) -> StdResult<Self> {
            self.0.$op(other).map(|val| Self(val))
        }
    };

    (fn $op:ident,arg $other:ident, => Self) => {
        fn $op(self, other: $other) -> Self {
            Self(self.0.$op(other))
        }
    };

    (fn $op:ident,field $inner:tt, => Result<Self>) => {
        fn $op(self, other: Self) -> StdResult<Self> {
            self.0.$op(other.$inner).map(|val| Self(val))
        }
    };

    (fn $op:ident,field $inner:tt, => Self) => {
        fn $op(self, other: Self) -> Self {
            Self(self.0.$op(other.$inner))
        }
    };

    (fn $op:ident, => Self) => {
        fn $op(self) -> Self {
            Self(self.0.$op())
        }
    };

    (fn $op:ident, => $out:ty) => {
        fn $op(self) -> $out {
            self.0.$op()
        }
    };
}

/// Given that T == U is implemented, also implement &T == U and T == &U.
/// Useful in creating math types.
///
/// Copied from CosmWasm:
/// <https://github.com/CosmWasm/cosmwasm/blob/v1.5.3/packages/std/src/forward_ref.rs>
#[macro_export]
macro_rules! forward_ref_partial_eq {
    ($t:ty, $u:ty) => {
        // &T == U
        impl<'a> PartialEq<$u> for &'a $t {
            #[inline]
            fn eq(&self, rhs: &$u) -> bool {
                **self == *rhs
            }
        }

        // T == &U
        impl PartialEq<&$u> for $t {
            #[inline]
            fn eq(&self, rhs: &&$u) -> bool {
                *self == **rhs
            }
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
macro_rules! forward_ref_binop_decimal {
    (impl $imp:ident, $method:ident for $t:ty, $u:ty) => {
        impl<U, const S: usize> std::ops::$imp<$u> for &'_ $t
        where
            $t: std::ops::$imp<$u> + Copy,
        {
            type Output = <$t as std::ops::$imp<$u>>::Output;

            #[inline]
            fn $method(self, other: $u) -> <$t as std::ops::$imp<$u>>::Output {
                std::ops::$imp::$method(*self, other)
            }
        }

        impl<U, const S: usize> std::ops::$imp<&$u> for $t
        where
            $t: std::ops::$imp<$u> + Copy,
        {
            type Output = <$t as std::ops::$imp<$u>>::Output;

            #[inline]
            fn $method(self, other: &$u) -> <$t as std::ops::$imp<$u>>::Output {
                std::ops::$imp::$method(self, *other)
            }
        }

        impl<U, const S: usize> std::ops::$imp<&$u> for &'_ $t
        where
            $t: std::ops::$imp<$u> + Copy,
        {
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

#[macro_export]
macro_rules! forward_ref_op_assign_decimal {
    (impl $imp:ident, $method:ident for $t:ty, $u:ty) => {
        impl<U, const S: usize> std::ops::$imp<&$u> for $t
        where
            $t: std::ops::$imp<$u> + Copy,
        {
            #[inline]
            fn $method(&mut self, other: &$u) {
                std::ops::$imp::$method(self, *other);
            }
        }
    };
}

#[macro_export]
macro_rules! generate_decimal_per {
    ($name:ident, $shift:expr) => {
        pub fn $name(x: impl Into<Int<U>>) -> Self {
            let atomic = x.into() * (Self::decimal_fraction() / Self::f_pow(($shift) as u32));
            Self::raw(atomic)
        }
    };
}

/// Generate `unchecked fn` from `checked fn`
#[macro_export]
macro_rules! generate_unchecked {
    ($checked:tt => $name:ident) => {
        pub fn $name(self) -> Self {
            self.$checked().unwrap()
        }
    };

    ($checked:tt => $name:ident,arg $arg:ident) => {
        pub fn $name(self, arg: $arg) -> Self {
            self.$checked(arg).unwrap()
        }
    };

    ($checked:tt => $name:ident,args $arg1:ty, $arg2:ty) => {
        pub fn $name(arg1: $arg1, arg2: $arg2) -> Self {
            Self::$checked(arg1, arg2).unwrap()
        }
    };
}
