use {
    data_encoding::BASE64,
    hex::FromHexError,
    std::{any::type_name, array::TryFromSliceError, convert::Infallible},
    thiserror::Error,
};

#[derive(Debug, Clone, Error)]
pub enum VerificationError {
    #[error("data is of incorrect length")]
    IncorrectLength,

    #[error("invalid recovery id; must be 0 or 1")]
    InvalidRecoveryId,

    #[error("signature is unauthentic")]
    Unauthentic,
}

impl VerificationError {
    /// Convert the error code received across WebAssembly FFI into a
    /// `VerificationError`.
    pub fn from_error_code(error_code: u32) -> Self {
        match error_code {
            1 => Self::IncorrectLength,
            2 => Self::InvalidRecoveryId,
            3 => Self::Unauthentic,
            _ => unreachable!("unknown verification error code: {error_code}, must be 1-3"),
        }
    }
}

#[derive(Debug, Clone, Error)]
pub enum StdError {
    /// This variant exists such that we can use `Coins` as the generic `C` in
    /// contructor methods `Message::{instantiate,execute}`, which has the trait
    /// bound: `StdError: From<<C as TryInto<Coins>>::Error>`.
    #[error(transparent)]
    Infallible(#[from] Infallible),

    #[error(transparent)]
    FromHex(#[from] FromHexError),

    #[error(transparent)]
    TryFromSlice(#[from] TryFromSliceError),

    // TODO: rename this. this means an error is thrown by the host over the FFI.
    // something like `StdError::Host` may be more appropriate.
    #[error("generic error: {0}")]
    Generic(String),

    #[error(transparent)]
    Verification(#[from] VerificationError),

    #[error("out of gas! limit: {limit}, used: {used}, comment: {comment}")]
    OutOfGas {
        limit: u64,
        used: u64,
        comment: &'static str,
    },

    #[error("failed to parse string `{value}` into {ty}: {reason}")]
    ParseNumber {
        ty: &'static str,
        value: String,
        reason: String,
    },

    #[error("invalid coins: {reason}")]
    InvalidCoins { reason: String },

    #[error("invalid payment: expecting {expect} coins, found {actual}")]
    InvalidPayment { expect: usize, actual: usize },

    #[error("cannot find denom `{denom}` in coins")]
    DenomNotFound { denom: String },

    #[error("data not found! type: {ty}, storage key: {key}")]
    DataNotFound { ty: &'static str, key: String },

    #[error("duplicate data found! type: {ty}, data: {data}")]
    DuplicateData { ty: &'static str, data: String },

    #[error("cannot find iterator with ID {iterator_id}")]
    IteratorNotFound { iterator_id: i32 },

    #[error("conversion overflow: {source_type}({value}) > {target_type}::MAX")]
    OverflowConversion {
        source_type: &'static str,
        target_type: &'static str,
        value: String,
    },

    #[error("addition overflow: {a} + {b} > {ty}::MAX")]
    OverflowAdd {
        ty: &'static str,
        a: String,
        b: String,
    },

    #[error("subtraction overflow: {a} - {b} < {ty}::MIN")]
    OverflowSub {
        ty: &'static str,
        a: String,
        b: String,
    },

    #[error("multiplication overflow: {a} * {b} > {ty}::MAX")]
    OverflowMul {
        ty: &'static str,
        a: String,
        b: String,
    },

    #[error("power overflow: {a} ^ {b} > {ty}::MAX")]
    OverflowPow {
        ty: &'static str,
        a: String,
        b: String,
    },

    #[error("left shift overflow: {a} << {b}")]
    OverflowShl { a: String, b: u32 },

    #[error("right shift overflow: {a} >> {b}")]
    OverflowShr { a: String, b: u32 },

    #[error("division by zero: {a} / 0")]
    DivisionByZero { a: String },

    #[error("remainder by zero: {a} % 0")]
    RemainderByZero { a: String },

    #[error("multiply a non-negative lhs with a negative rhs: {ty}({a}) * {b}")]
    NegativeMul {
        ty: &'static str,
        a: String,
        b: String,
    },

    #[error("divide a non-negative lhs with a negative rhs: {ty}({a}) / {b}")]
    NegativeDiv {
        ty: &'static str,
        a: String,
        b: String,
    },

    #[error("square root of negative: sqrt({a})")]
    NegativeSqrt { a: String },

    #[error("logarithm of zero")]
    ZeroLog,

    #[error("expecting a non-zero value of type {ty}, got zero")]
    ZeroValue { ty: &'static str },

    #[error("failed to serialize into json! codec: {codec}, type: {ty}, reason: {reason}")]
    Serialize {
        codec: &'static str,
        ty: &'static str,
        reason: String,
    },

    #[error("failed to deserialize from json! codec: {codec}, type: {ty}, reason: {reason}")]
    Deserialize {
        codec: &'static str,
        ty: &'static str,
        reason: String,
    },
}

impl StdError {
    pub fn parse_number<T, V, R>(value: V, reason: R) -> Self
    where
        V: ToString,
        R: ToString,
    {
        Self::ParseNumber {
            ty: type_name::<T>(),
            value: value.to_string(),
            reason: reason.to_string(),
        }
    }

    pub fn invalid_coins<R>(reason: R) -> Self
    where
        R: ToString,
    {
        Self::InvalidCoins {
            reason: reason.to_string(),
        }
    }

    pub fn invalid_payment(expect: usize, actual: usize) -> Self {
        Self::InvalidPayment { expect, actual }
    }

    pub fn data_not_found<T>(key: &[u8]) -> Self {
        Self::DataNotFound {
            ty: type_name::<T>(),
            key: BASE64.encode(key),
        }
    }

    pub fn duplicate_data<T>(data: &[u8]) -> Self {
        Self::DuplicateData {
            ty: type_name::<T>(),
            data: BASE64.encode(data),
        }
    }

    pub fn overflow_conversion<A, B>(source: A) -> Self
    where
        A: ToString,
    {
        Self::OverflowConversion {
            source_type: type_name::<A>(),
            target_type: type_name::<B>(),
            value: source.to_string(),
        }
    }

    pub fn overflow_add<T>(a: T, b: T) -> Self
    where
        T: ToString,
    {
        Self::OverflowAdd {
            ty: type_name::<T>(),
            a: a.to_string(),
            b: b.to_string(),
        }
    }

    pub fn overflow_sub<T>(a: T, b: T) -> Self
    where
        T: ToString,
    {
        Self::OverflowSub {
            ty: type_name::<T>(),
            a: a.to_string(),
            b: b.to_string(),
        }
    }

    pub fn overflow_mul<T>(a: T, b: T) -> Self
    where
        T: ToString,
    {
        Self::OverflowMul {
            ty: type_name::<T>(),
            a: a.to_string(),
            b: b.to_string(),
        }
    }

    pub fn overflow_pow<T>(a: T, b: u32) -> Self
    where
        T: ToString,
    {
        Self::OverflowPow {
            ty: type_name::<T>(),
            a: a.to_string(),
            b: b.to_string(),
        }
    }

    pub fn overflow_shl<T>(a: T, b: u32) -> Self
    where
        T: ToString,
    {
        Self::OverflowShl {
            a: a.to_string(),
            b,
        }
    }

    pub fn overflow_shr<T>(a: T, b: u32) -> Self
    where
        T: ToString,
    {
        Self::OverflowShr {
            a: a.to_string(),
            b,
        }
    }

    pub fn division_by_zero<T>(a: T) -> Self
    where
        T: ToString,
    {
        Self::DivisionByZero { a: a.to_string() }
    }

    pub fn remainder_by_zero<T>(a: T) -> Self
    where
        T: ToString,
    {
        Self::RemainderByZero { a: a.to_string() }
    }

    pub fn zero_log() -> Self {
        Self::ZeroLog
    }

    pub fn zero_value<T>() -> Self {
        Self::ZeroValue {
            ty: type_name::<T>(),
        }
    }

    pub fn negative_mul<A, B>(a: A, b: B) -> Self
    where
        A: ToString,
        B: ToString,
    {
        Self::NegativeMul {
            ty: type_name::<A>(),
            a: a.to_string(),
            b: b.to_string(),
        }
    }

    pub fn negative_div<A, B>(a: A, b: B) -> Self
    where
        A: ToString,
        B: ToString,
    {
        Self::NegativeDiv {
            ty: type_name::<A>(),
            a: a.to_string(),
            b: b.to_string(),
        }
    }

    pub fn negative_sqrt<T>(a: T) -> Self
    where
        T: ToString,
    {
        Self::NegativeSqrt { a: a.to_string() }
    }

    pub fn serialize<T, R>(codec: &'static str, reason: R) -> Self
    where
        R: ToString,
    {
        Self::Serialize {
            codec,
            ty: type_name::<T>(),
            reason: reason.to_string(),
        }
    }

    pub fn deserialize<T, R>(codec: &'static str, reason: R) -> Self
    where
        R: ToString,
    {
        Self::Deserialize {
            codec,
            ty: type_name::<T>(),
            reason: reason.to_string(),
        }
    }

    pub fn generic_err<R>(reason: R) -> Self
    where
        R: ToString,
    {
        Self::Generic(reason.to_string())
    }
}

pub type StdResult<T> = core::result::Result<T, StdError>;
