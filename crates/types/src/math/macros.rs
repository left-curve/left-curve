/// Generate a [`Unit`](super::Uint) type for a given inner type.
///
/// ### Example:
/// ```ignore
/// generate_int!(
///     // The name of the Uint
///     name = Uint128,     
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
///     // If ibnum, impl_bytable_ibnum be will call. unsigned has to be specify
///     // If skipped, no impl_bytable will be called.
///     // This require to implement the Bytable trait manually
///     impl_bytable = std,
///     // Implement From | TryInto from other types
///     from = [Uint64]
/// );
#[macro_export]
macro_rules! generate_int {
        // impl_bytable = std
        (
            name = $name:ident,
            inner_type = $inner:ty,
            min = $min:expr,
            max = $max:expr,
            zero = $zero:expr,
            one = $one:expr,
            ten = $ten:expr,
            byte_len = $byte_len:literal,
            impl_bytable = std,
            from = [$($from:ty),*]
        ) => {
            impl_bytable_std!($inner, $byte_len);
            generate_int!(
                name = $name,
                inner_type = $inner,
                min = $min,
                max = $max,
                zero = $zero,
                one = $one,
                ten = $ten,
                byte_len = $byte_len,
                from = [$($from),*]
            );
        };
        // impl_bytable = bnum
        (
            name = $name:ident,
            inner_type = $inner:ty,
            min = $min:expr,
            max = $max:expr,
            zero = $zero:expr,
            one = $one:expr,
            ten = $ten:expr,
            byte_len = $byte_len:literal,
            impl_bytable = bnum,
            from = [$($from:ty),*]
        ) => {
            impl_bytable_bnum!($inner, $byte_len);
            generate_int!(
                name = $name,
                inner_type = $inner,
                min = $min,
                max = $max,
                zero = $zero,
                one = $one,
                ten = $ten,
                byte_len = $byte_len,
                from = [$($from),*]
            );
        };
        // impl_bytable = ibnum
        (
            name = $name:ident,
            inner_type = $inner:ty,
            min = $min:expr,
            max = $max:expr,
            zero = $zero:expr,
            one = $one:expr,
            ten = $ten:expr,
            byte_len = $byte_len:literal,
            impl_bytable = ibnum unsigned $unsigned:ty,
            from = [$($from:ty),*]
        ) => {
            impl_bytable_ibnum!($inner, $byte_len, $unsigned);
            generate_int!(
                name = $name,
                inner_type = $inner,
                min = $min,
                max = $max,
                zero = $zero,
                one = $one,
                ten = $ten,
                byte_len = $byte_len,
                from = [$($from),*]
            );
        };
        // impl_bytable = none (Optional)
        (
            name = $name:ident,
            inner_type = $inner:ty,
            min = $min:expr,
            max = $max:expr,
            zero = $zero:expr,
            one = $one:expr,
            ten = $ten:expr,
            byte_len = $byte_len:literal,
            from = [$($from:ty),*]

        ) => {
            impl_number_bound!($inner, $max, $min, $zero, $one, $ten);
            impl_checked_ops_unsigned!($inner);
            pub type $name = Uint<$inner>;

            // Impl From Uint and from inner type
            $(
                impl From<$from> for $name {
                    fn from(value: $from) -> Self {
                        let others_byte = value.to_le_bytes();
                        let mut bytes: [u8; $byte_len] = [0; $byte_len];
                        for i in 0..others_byte.len() {
                            bytes[i] = others_byte[i];
                        }
                        Self::from_le_bytes(bytes)
                    }
                }

                impl From<<$from as UintInner>::U> for $name {
                    fn from(value: <$from as UintInner>::U) -> Self {
                        let others_byte = value.to_le_bytes();
                        let mut bytes: [u8; $byte_len] = [0; $byte_len];
                        for i in 0..others_byte.len() {
                            bytes[i] = others_byte[i];
                        }
                        Self::from_le_bytes(bytes)
                    }
                }

                impl TryInto<$from> for $name {
                    type Error = StdError;

                    fn try_into(self) -> StdResult<$from> {
                        let other_b_l = <$from>::byte_len();
                        let bytes = self.to_le_bytes();
                        let (lower, higher) = bytes.split_at(other_b_l);
                        if higher.iter().any(|&b| b != 0) {
                            // TODO: Change this error after implementing FromStr for Uint
                            return Err(StdError::Generic("Conversion error!".to_string()));
                        }
                        Ok(<$from>::from_le_bytes(lower.try_into()?))
                    }

                }

                // TODO: Maybe generate a closure to avoid code duplication
                impl TryInto<<$from as UintInner>::U> for $name {
                    type Error = StdError;

                    fn try_into(self) -> StdResult<<$from as UintInner>::U> {
                        let other_b_l = <$from>::byte_len();
                        let bytes = self.to_le_bytes();
                        let (lower, higher) = bytes.split_at(other_b_l);
                        if higher.iter().any(|&b| b != 0) {
                            // TODO: Change this error after implementing FromStr for Uint
                            return Err(StdError::Generic("Conversion error!".to_string()));
                        }
                        Ok(<$from>::from_le_bytes(lower.try_into()?).into())
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
    ($t:ty, $max:expr, $min:expr, $zero:expr, $one:expr, $ten:expr) => {
        impl NumberConst for $t {
            const MAX: Self = $max;
            const MIN: Self = $min;
            const ZERO: Self = $zero;
            const ONE: Self = $one;
            const TEN: Self = $ten;
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
        }
    };
}

#[macro_export]
macro_rules! impl_checked_ops_unsigned {
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
            fn abs(self) -> Self {
                self
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
        // Decimal
        (impl Decimal with $imp:ident, $method:ident for $t:ty where sub fn $sub_method:ident) =>
        {
            impl<U, const S: usize> std::ops::$imp for $t
            where
                Uint<U>: NextNumber + CheckedOps,
                <Uint<U> as NextNumber>::Next: From<Uint<U>> + TryInto<Uint<U>> + CheckedOps + ToString + Clone,
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
macro_rules! call_inner {
    (fn $op:ident, arg $other:ident, => Result<Self>) => {
        fn $op(self, other: $other) -> StdResult<Self> {
            self.0.$op(other).map(|val| Self(val))
        }
    };
    (fn $op:ident, arg $other:ident, => Self) => {
        fn $op(self, other: $other) -> Self {
            Self(self.0.$op(other))
        }
    };

    (fn $op:ident, field $inner:tt, => Result<Self>) => {
        fn $op(self, other: Self) -> StdResult<Self> {
            self.0.$op(other.$inner).map(|val| Self(val))
        }
    };
    (fn $op:ident, field $inner:tt, => Self) => {
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

#[macro_export]
macro_rules! generate_decimal_per {
    ($name:ident, $shift:expr) => {
        pub fn $name(x: impl Into<Uint<U>>) -> Self {
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
    ($checked:tt => $name:ident, arg $arg:ident) => {
        pub fn $name(self, arg: $arg) -> Self {
            self.$checked(arg).unwrap()
        }
    };
    ($checked:tt => $name:ident, args $arg1:ty, $arg2:ty) => {
        pub fn $name(arg1: $arg1, arg2: $arg2) -> Self {
            Self::$checked(arg1, arg2).unwrap()
        }
    };
}
