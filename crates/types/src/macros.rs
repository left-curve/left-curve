// -------------------------- generic error handling ---------------------------

// TODO: replace with https://doc.rust-lang.org/std/ops/trait.Try.html once stabilized
#[macro_export]
#[doc(hidden)]
macro_rules! unwrap_into_generic_result {
    ($expr:expr) => {
        match $expr {
            Ok(val) => val,
            Err(err) => {
                return GenericResult::Err(err.to_string());
            },
        }
    };
}

// ---------------------------------- context ----------------------------------

#[macro_export]
#[doc(hidden)]
#[rustfmt::skip]
macro_rules! make_immutable_ctx {
    ($ctx:ident, $storage:expr, $api:expr, $querier:expr) => {
        {
            debug_assert!($ctx.sender.is_none());
            debug_assert!($ctx.funds.is_none());
            debug_assert!($ctx.mode.is_none());

            ImmutableCtx {
                storage:  $storage,
                api:      $api,
                querier:  QuerierWrapper::new($querier),
                chain_id: $ctx.chain_id,
                block:    $ctx.block,
                contract: $ctx.contract,
            }
        }
    };
}

#[macro_export]
#[doc(hidden)]
#[rustfmt::skip]
macro_rules! make_mutable_ctx {
    ($ctx:ident, $storage:expr, $api:expr, $querier:expr) => {
        {
            debug_assert!($ctx.mode.is_none());

            MutableCtx {
                storage:  $storage,
                api:      $api,
                querier:  QuerierWrapper::new($querier),
                chain_id: $ctx.chain_id,
                block:    $ctx.block,
                contract: $ctx.contract,
                sender:   $ctx.sender.unwrap(),
                funds:    $ctx.funds.unwrap(),
            }
        }
    };
}

#[macro_export]
#[doc(hidden)]
#[rustfmt::skip]
macro_rules! make_sudo_ctx {
    ($ctx:ident, $storage:expr, $api:expr, $querier:expr) => {
        {
            debug_assert!($ctx.sender.is_none());
            debug_assert!($ctx.funds.is_none());
            debug_assert!($ctx.mode.is_none());

            SudoCtx {
                storage:  $storage,
                api:      $api,
                querier:  QuerierWrapper::new($querier),
                chain_id: $ctx.chain_id,
                block:    $ctx.block,
                contract: $ctx.contract,
            }
        }
    };
}

#[macro_export]
#[doc(hidden)]
#[rustfmt::skip]
macro_rules! make_auth_ctx {
    ($ctx:ident, $storage:expr, $api:expr, $querier:expr) => {
        {
            debug_assert!($ctx.sender.is_none());
            debug_assert!($ctx.funds.is_none());

            AuthCtx {
                storage:  $storage,
                api:      $api,
                querier:  QuerierWrapper::new($querier),
                chain_id: $ctx.chain_id,
                block:    $ctx.block,
                contract: $ctx.contract,
                mode:     $ctx.mode.unwrap(),
            }
        }
    };
}

// ----------------------------------- math ------------------------------------

// Macros that are mainly used to define math types. They are generally not
// intended for use outside of this crate.

/// Generate a [`Uint`](super::Uint) type for a given inner type.
///
/// ### Example
///
/// ```ignore
/// generate_uint!(
///     // The name of the Uint
///     name = Uint128,
///     // Inner type of the Uint
///     inner_type = u128,
///     // Implement From | TryFrom from other Uint types
///     // Safe type where overflow is not possible
///     // It also impls Base ops (Add, Sub ecc..) vs this type
///     from_int = [Uint64],
///     // Implement From | TryFrom from other std types
///     // Safe type where overflow is not possible
///     // It also impls Base ops (Add, Sub ecc..) vs this type
///     from_std = [u128, u32, u16, u8],
///     // Implement TryFrom | TryFrom from other Uint types
///     // Unsafe type where overflow is possible
///     try_from_int = [Int128],
/// );
#[macro_export]
#[doc(hidden)]
macro_rules! generate_uint {
    (
        name = $name:ident,
        inner_type = $inner:ty,
        from_int = [$($from:ty),*],
        from_std = [$($from_std:ty),*],
        doc = $doc:literal,
    ) => {
        #[doc = $doc]
        pub type $name = Uint<$inner>;

        forward_ref_partial_eq!($name, $name);

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
                type Error = StdError;
                fn try_from(value: $name) -> StdResult<$from> {
                    <$from as Inner>::U::try_from(value.number())
                        .map(Self)
                        .map_err(|_| StdError::overflow_conversion::<_, $from>(value))
                }
            }

            // Ex: TryFrom<Uint128> for u64
            impl TryFrom<$name> for <$from as Inner>::U {
                type Error = StdError;
                fn try_from(value: $name) -> StdResult<<$from as Inner>::U> {
                    <$from as Inner>::U::try_from(value.number())
                        .map_err(|_| StdError::overflow_conversion::<_, $from>(value))
                }
            }

            impl_all_ops_and_assign!($name, $from);
            impl_all_ops_and_assign!($name, <$from as Inner>::U);
        )*

        // --- Impl From std ---
        $(
            // Ex: From<u32> for Uint128
            impl From<$from_std> for $name {
                fn from(value: $from_std) -> Self {
                    Self::new_from(value)
                }
            }

            // Ex: TryFrom<Uint128> for u32
            impl TryFrom<$name> for $from_std {
                type Error = StdError;
                fn try_from(value: $name) -> StdResult<$from_std> {
                    <$from_std>::try_from(value.number())
                    .map_err(|_| StdError::overflow_conversion::<_, $from_std>(value))
                }
            }

            // --- Impl ops ---
            impl_all_ops_and_assign!($name, $from_std);
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

/// Generate a [`Udec`](super::Udec) type for a given inner type.
///
/// ### Example
///
/// ```ignore
/// generate_uint!(
///     // The name of the Uint
///     name = Udec256,
///     // Inner type of the Uint
///     inner_type = Uint256,
///     // Number of decimal places
///     decimal_places = 18,
///     // Implement From | TryFrom from other Udec types
///     // Safe type where overflow is not possible
///     // It also impls Base ops (Add, Sub ecc..) vs this type
///     from_dec = [Dec128, Udec128],
/// );
#[macro_export]
#[doc(hidden)]
macro_rules! generate_decimal {
    (
        name = $name:ident,
        inner_type = $inner:ty,
        decimal_places = $decimal_places:expr,
        from_dec = [$($from:ty),*],
        doc = $doc:literal,
    ) => {
        #[doc = $doc]
        pub type $name = Udec<$inner, $decimal_places>;

        // Ex: From<U256> for Udec256
        impl From<$inner> for $name {
            fn from(value: $inner) -> Self {
                Self::raw(Uint::new(value))
            }
        }

        // Ex: From<Uint<U256>> for Udec256
        impl From<Uint<$inner>> for $name {
            fn from(value: Uint<$inner>) -> Self {
                Self::raw(value)
            }
        }

        // --- From Udec ---
        $(
            // Ex: From<Udec128> for Udec256
            impl From<$from> for $name {
                fn from(value: $from) -> Self {
                    Self::from_decimal(value)
                }
            }

            // Ex: From<Uint128> for Udec256
            impl From<Uint<<$from as Inner>::U>> for $name {
                fn from(value: Uint<<$from as Inner>::U>) -> Self {
                    Self::raw(value.into())
                }
            }

            // Ex: From<u128> for Udec256
            impl From<<$from as Inner>::U> for $name {
                fn from(value: <$from as Inner>::U) -> Self {
                    Self::raw(value.into())
                }
            }

            // Ex: TryFrom<Udec256> for Udec128
            impl TryFrom<$name> for $from {
                type Error = StdError;

                fn try_from(value: $name) -> StdResult<$from> {
                    <$from>::try_from_decimal(value)
                }
            }

            // Ex: TryFrom<Udec256> for Uint128
            impl TryFrom<$name> for Uint<<$from as Inner>::U> {
                type Error = StdError;

                fn try_from(value: $name) -> StdResult<Uint<<$from as Inner>::U>> {
                    value.0.try_into().map(Self)
                }
            }

            // Ex: TryFrom<Udec256> for u128
            impl TryFrom<$name> for <$from as Inner>::U {
                type Error = StdError;

                fn try_from(value: $name) -> StdResult<<$from as Inner>::U> {
                    value.0.try_into()
                }
            }

            impl_all_ops_and_assign!($name, $from);
        )*
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! generate_signed {
    (
        name = $name:ident,
        inner_type = $inner:ty,
        from_signed = [$($from_signed:ty),*],
        from_std = [$($from_std:ty),*],
        doc = $doc:literal,
    ) => {
        #[doc = $doc]
        pub type $name = Signed<$inner>;

        // Ex: From<Uint128> for Int128
        impl From<$inner> for $name {
            fn from(value: $inner) -> Self {
                Self::new_positive(value)
            }
        }

        // Ex: From<u128> for Int128
        impl From<<$inner as Inner>::U> for $name {
            fn from(value: <$inner as Inner>::U) -> Self {
                Self::new_positive(<$inner>::new(value))
            }
        }

        // Ex: TyFrom<Int128> for Uint128
        impl TryFrom<$name> for $inner {
            type Error = StdError;

            fn try_from(value: $name) -> StdResult<Self> {
                if value.is_negative() {
                    Err(StdError::overflow_conversion::<_, $inner>(value))
                } else {
                    Ok(value.abs)
                }
            }
        }

        impl_all_ops_and_assign!($name, $inner);
        impl_all_ops_and_assign!($name, <$inner as Inner>::U);

        // --- From other signed types ---
        $(
            // Ex: From<Int64> for Int128
            impl From<$from_signed> for $name {
                fn from(value: $from_signed) -> Self {
                    Self::new(value.abs.into(), value.is_negative())
                }
            }

            // Ex: From<Uint64> for Int128
            impl From<<$from_signed as Inner>::U> for $name {
                fn from(value: <$from_signed as Inner>::U) -> Self {
                    Self::new_positive(value.into())
                }
            }

            // Ex: From<u64> for Int128
            impl From<<<$from_signed as Inner>::U as Inner>::U> for $name {
                fn from(value: <<$from_signed as Inner>::U as Inner>::U) -> Self {
                    Self::new_positive(value.into())
                }
            }

            // Ex: TryFrom<Int128> for Int64
            impl TryFrom<$name> for $from_signed {
                type Error = StdError;

                fn try_from(value: $name) -> StdResult<$from_signed> {
                    <$from_signed as Inner>::U::try_from(value.abs)
                        .map(|val| Self::new(val, value.is_negative()))
                        .map_err(|_| StdError::overflow_conversion::<_, $from_signed>(value))
                }
            }

            // Ex: TryFrom<Int128> for Uint64
            impl TryFrom<$name> for <$from_signed as Inner>::U {
                type Error = StdError;

                fn try_from(value: $name) -> StdResult<<$from_signed as Inner>::U> {
                    if value.is_negative() {
                        return Err(StdError::overflow_conversion::<_, $name>(value))
                    }
                    <$from_signed as Inner>::U::try_from(value.abs)
                        .map_err(|_| StdError::overflow_conversion::<_, $from_signed>(value))
                }
            }

            // Ex: TryFrom<Int128> for u64
            impl TryFrom<$name> for <<$from_signed as Inner>::U as Inner>::U {
                type Error = StdError;

                fn try_from(value: $name) -> StdResult<<<$from_signed as Inner>::U as Inner>::U> {
                    if value.is_negative() {
                        return Err(StdError::overflow_conversion::<_, $name>(value))
                    }
                    <<$from_signed as Inner>::U as Inner>::U::try_from(value.abs)
                        .map_err(|_| StdError::overflow_conversion::<_, $from_signed>(value))
                }
            }

            impl_all_ops_and_assign!($name, $from_signed);
            impl_all_ops_and_assign!($name, <$from_signed as Inner>::U);
            impl_all_ops_and_assign!($name, <<$from_signed as Inner>::U as Inner>::U);
        )*

        // --- From std ---
        $(
            // Ex: From<u32> for Int128
            impl From<$from_std> for $name {
                fn from(value: $from_std) -> Self {
                    Self::new_positive(value.into())
                }
            }

            // Ex: TryFrom<Int128> for u32
            impl TryFrom<$name> for $from_std {
                type Error = StdError;

                fn try_from(value: $name) -> StdResult<$from_std> {
                    <$from_std>::try_from(value.abs)
                        .map_err(|_| StdError::overflow_conversion::<_, $from_std>(value))
                }
            }

            impl_all_ops_and_assign!($name, $from_std);
        )*
    };

    // --- From std ---
}

/// **Syntax**:
///
/// ```ignore
/// impl_number_const!(type, max, min, zero, one);
/// ```
///
/// **Example**:
///
/// ```ignore
/// impl_number_const!(u64, u64::MAX, u64::MIN, 0, 1);
/// ```
#[macro_export]
#[doc(hidden)]
macro_rules! impl_number_const {
    ($t:ty, $min:expr, $max:expr, $zero:expr, $one:expr, $ten:expr) => {
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
#[doc(hidden)]
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
#[doc(hidden)]
macro_rules! impl_bytable_bnum {
    ($t:ty, $rot:literal) => {
        impl Bytable<$rot> for $t {
            fn from_be_bytes(bytes: [u8; $rot]) -> Self {
                let mut digits = [0u64; $rot / 8];
                for i in 0..$rot / 8 {
                    digits[i] = u64::from_le_bytes([
                        bytes[($rot / 8 - i - 1) * 8 + 7],
                        bytes[($rot / 8 - i - 1) * 8 + 6],
                        bytes[($rot / 8 - i - 1) * 8 + 5],
                        bytes[($rot / 8 - i - 1) * 8 + 4],
                        bytes[($rot / 8 - i - 1) * 8 + 3],
                        bytes[($rot / 8 - i - 1) * 8 + 2],
                        bytes[($rot / 8 - i - 1) * 8 + 1],
                        bytes[($rot / 8 - i - 1) * 8],
                    ]);
                }
                Self::from_digits(digits)
            }

            fn from_le_bytes(bytes: [u8; $rot]) -> Self {
                let mut digits = [0u64; $rot / 8];
                for i in 0..$rot / 8 {
                    digits[i] = u64::from_le_bytes([
                        bytes[i * 8],
                        bytes[i * 8 + 1],
                        bytes[i * 8 + 2],
                        bytes[i * 8 + 3],
                        bytes[i * 8 + 4],
                        bytes[i * 8 + 5],
                        bytes[i * 8 + 6],
                        bytes[i * 8 + 7],
                    ]);
                }
                Self::from_digits(digits)
            }

            fn to_be_bytes(self) -> [u8; $rot] {
                let words = self.digits();
                let mut bytes = [[0u8; 8]; $rot / 8];
                for i in 0..$rot / 8 {
                    bytes[i] = words[$rot / 8 - i - 1].to_be_bytes();
                }

                unsafe { std::mem::transmute(bytes) }
            }

            fn to_le_bytes(self) -> [u8; $rot] {
                let words = self.digits();
                let mut bytes = [[0u8; 8]; $rot / 8];
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
#[doc(hidden)]
macro_rules! impl_integer_number {
    ($t:ty) => {
        // This macro is only applied to unsigned number types (`u64`,
        // `u128`, `U256`, `U512`) which cannot be negative.
        impl Sign for $t {
            fn abs(self) -> Self {
                self
            }

            fn is_negative(&self) -> bool {
                false
            }
        }

        impl Number for $t
        where
            $t: NumberConst,
        {
            fn is_zero(&self) -> bool {
                *self == Self::ZERO
            }

            fn abs(self) -> Self {
                // This macro is only applied to unsigned number types (`u64`,
                // `u128`, `U256`, `U512`) for which the absolute values are
                // simply themselves.
                self
            }

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

            /// Compute a _positive_ integer's _floored_ square root using the
            /// [Babylonian method](https://en.wikipedia.org/wiki/Methods_of_computing_square_roots#Heron's_method).
            fn checked_sqrt(self) -> StdResult<Self> {
                if self.is_zero() {
                    return Ok(Self::ZERO);
                }
                let mut x = self;
                let mut y = (x + 1) >> 1;
                while y < x {
                    x = y;
                    y = (x + self / x) >> 1;
                }
                Ok(x)
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
        }

        impl Integer for $t {
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
        }
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! impl_next {
    ($t:ty, $next:ty) => {
        impl NextNumber for $t {
            type Next = $next;
        }
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! impl_all_ops_and_assign {
    ($name:ident, $other:ty) => {
        impl_number!(impl Add, add for $name as $other where sub fn checked_add);
        impl_number!(impl Sub, sub for $name as $other where sub fn checked_sub);
        impl_number!(impl Mul, mul for $name as $other where sub fn checked_mul);
        impl_number!(impl Div, div for $name as $other where sub fn checked_div);

        forward_ref_binop!(impl Add, add for $name, $other);
        forward_ref_binop!(impl Sub, sub for $name, $other);
        forward_ref_binop!(impl Mul, mul for $name, $other);
        forward_ref_binop!(impl Div, div for $name, $other);

        forward_ref_binop!(impl Add, add for $other, $name);
        forward_ref_binop!(impl Sub, sub for $other, $name);
        forward_ref_binop!(impl Mul, mul for $other, $name);
        forward_ref_binop!(impl Div, div for $other, $name);

        impl_assign_number!(impl AddAssign, add_assign for $name as $other where sub fn checked_add);
        impl_assign_number!(impl SubAssign, sub_assign for $name as $other where sub fn checked_sub);
        impl_assign_number!(impl MulAssign, mul_assign for $name as $other where sub fn checked_mul);
        impl_assign_number!(impl DivAssign, div_assign for $name as $other where sub fn checked_div);

        forward_ref_op_assign!(impl AddAssign, add_assign for $name, $other);
        forward_ref_op_assign!(impl SubAssign, sub_assign for $name, $other);
        forward_ref_op_assign!(impl MulAssign, mul_assign for $name, $other);
        forward_ref_op_assign!(impl DivAssign, div_assign for $name, $other);
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! impl_number {
    // args type = Self
    (impl<$($gen:tt),*> $imp:ident, $method:ident for $t:ty where sub fn $sub_method:ident) => {
        impl<$($gen),*> std::ops::$imp for $t
        where
            $t: Number,
        {
            type Output = Self;

            fn $method(self, other: Self) -> Self {
                self.$sub_method(other).unwrap_or_else(|err| panic!("{err}"))
            }
        }
    };

    // Ops self for other, output = Self
    // Ex: Add<Uint64> for Uint128 => Uint128
    // Ex: Add<Udec128> for Udec256 => Udec256
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
                other.$sub_method(self.into()).unwrap_or_else(|err| panic!("{err}"))
            }
        }
    };

    // Udec Self
    (impl Udec with $imp:ident, $method:ident for $t:ty where sub fn $sub_method:ident) => {
        impl<U, const S: u32> std::ops::$imp for $t
        where
            Self: Number,
        {
            type Output = Self;

            fn $method(self, other: Self) -> Self {
                self.$sub_method(other).unwrap_or_else(|err| panic!("{err}"))
            }
        }
    };

    // Signed
    (impl Signed with $imp:ident, $method:ident for $t:ty where sub fn $sub_method:ident) => {
        impl<T> std::ops::$imp for $t
        where
            Self: Number,
        {
            type Output = Self;

            fn $method(self, other: Self) -> Self {
                self.$sub_method(other).unwrap_or_else(|err| panic!("{err}"))
            }
        }
    };

}

#[macro_export]
#[doc(hidden)]
macro_rules! impl_integer {
    // args type = other
    (impl<$($gen:tt),*> $imp:ident, $method:ident for $t:ty where sub fn $sub_method:ident, $other:ty) => {
        impl<$($gen),*> std::ops::$imp<$other> for $t
        where
            $t: Integer,
        {
            type Output = Self;

            fn $method(self, other: $other) -> Self {
                self.$sub_method(other).unwrap_or_else(|err| panic!("{err}"))
            }
        }
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! impl_assign_number {
    // args type = Self
    (impl<$($gen:tt),*> $imp:ident, $method:ident for $t:ty where sub fn $sub_method:ident) => {
        impl<$($gen),*>core::ops::$imp for $t
        where
            $t: Number + Copy,
        {
            fn $method(&mut self, other: Self) {
                *self = (*self).$sub_method(other).unwrap_or_else(|err| panic!("{err}"))
            }
        }
    };

    // Udec
    (impl Udec with $imp:ident, $method:ident for $t:ty where sub fn $sub_method:ident) => {
        impl<U, const S: u32> std::ops::$imp for $t
        where
            Self: Number + Copy,
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

    // Signed
    (impl Signed with $imp:ident, $method:ident for $t:ty where sub fn $sub_method:ident) => {
        impl<T> std::ops::$imp for $t
        where
            Self: Number + Copy,
        {
            fn $method(&mut self, other: Self) {
                *self = (*self).$sub_method(other).unwrap_or_else(|err| panic!("{err}"))
            }
        }
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! impl_assign_integer {
    // args type = other
    (
        impl <
        $($gen:tt),* >
        $imp:ident,
        $method:ident for
        $t:ty where sub fn
        $sub_method:ident,
        $other:ty
    ) => {
        impl<U> core::ops::$imp<$other> for $t
        where
            $t: Integer + Copy,
        {
            fn $method(&mut self, other: $other) {
                *self = (*self)
                    .$sub_method(other)
                    .unwrap_or_else(|err| panic!("{err}"))
            }
        }
    };
}

/// Given that T == U is implemented, also implement &T == U and T == &U.
/// Useful in creating math types.
///
/// Copied from CosmWasm:
/// <https://github.com/CosmWasm/cosmwasm/blob/v1.5.3/packages/std/src/forward_ref.rs>
#[macro_export]
#[doc(hidden)]
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
#[doc(hidden)]
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
#[doc(hidden)]
macro_rules! forward_ref_binop_decimal {
    (impl $imp:ident, $method:ident for $t:ty, $u:ty) => {
        impl<U, const S: u32> std::ops::$imp<$u> for &'_ $t
        where
            $t: std::ops::$imp<$u> + Copy,
        {
            type Output = <$t as std::ops::$imp<$u>>::Output;

            #[inline]
            fn $method(self, other: $u) -> <$t as std::ops::$imp<$u>>::Output {
                std::ops::$imp::$method(*self, other)
            }
        }

        impl<U, const S: u32> std::ops::$imp<&$u> for $t
        where
            $t: std::ops::$imp<$u> + Copy,
        {
            type Output = <$t as std::ops::$imp<$u>>::Output;

            #[inline]
            fn $method(self, other: &$u) -> <$t as std::ops::$imp<$u>>::Output {
                std::ops::$imp::$method(self, *other)
            }
        }

        impl<U, const S: u32> std::ops::$imp<&$u> for &'_ $t
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
#[doc(hidden)]
macro_rules! forward_ref_op_assign_typed {
    (impl<$($gen:tt),*> $imp:ident, $method:ident for $t:ty, $u:ty) => {
        impl <$($gen),*> std::ops::$imp<&$u> for $t
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
#[doc(hidden)]
macro_rules! forward_ref_op_assign_decimal {
    (impl $imp:ident, $method:ident for $t:ty, $u:ty) => {
        impl<U, const S: u32> std::ops::$imp<&$u> for $t
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
