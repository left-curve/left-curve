use {
    crate::Denom,
    data_encoding::{BASE64, DecodeError},
    grug_math::MathError,
    std::{any::type_name, array::TryFromSliceError, convert::Infallible},
};

#[grug_macros::backtrace(grug_types_base)]
#[derive(Clone)]
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
            1 => Self::incorrect_length(),
            2 => Self::invalid_recovery_id(),
            3 => Self::unauthentic(),
            _ => unreachable!("unknown verification error code: {error_code}, must be 1-3"),
        }
    }
}

#[grug_macros::backtrace(grug_types_base)]
#[derive(Clone)]
pub enum StdError {
    #[error(transparent)]
    #[backtrace(new)]
    Infallible(Infallible),

    #[error(transparent)]
    #[backtrace(new)]
    TryFromSlice(TryFromSliceError),

    #[error(transparent)]
    #[backtrace(new)]
    Decode(DecodeError),

    #[error(transparent)]
    Math(MathError),

    #[error(transparent)]
    Verification(VerificationError),

    #[error("host returned error: {0}")]
    #[backtrace(private_constructor)]
    #[backtrace(new)]
    Host(String),

    #[error("invalid denom `{denom}`: {reason}")]
    #[backtrace(private_constructor)]
    InvalidDenom { denom: String, reason: &'static str },

    #[error("invalid coins: {reason}")]
    #[backtrace(private_constructor)]
    InvalidCoins { reason: String },

    #[error("invalid payment: expecting {expect}, found {actual}")]
    #[backtrace(private_constructor)]
    InvalidPayment { expect: String, actual: String },

    #[error("cannot find denom `{denom}` in coins")]
    DenomNotFound { denom: Denom },

    #[error("data not found! type: {ty}, storage key: {key}")]
    #[backtrace(private_constructor)]
    DataNotFound { ty: &'static str, key: String },

    #[error("duplicate data found! type: {ty}")]
    #[backtrace(private_constructor)]
    DuplicateData { ty: &'static str },

    #[error("expecting a non-empty value of type {ty}, got empty")]
    #[backtrace(private_constructor)]
    EmptyValue { ty: &'static str },

    #[error("expecting a non-zero value of type {ty}, got zero")]
    #[backtrace(private_constructor)]
    ZeroValue { ty: &'static str },

    #[error("invalid change set: the add and remove sets must be disjoint")]
    InvalidChangeSet,

    #[error("value out of range: {value} {comparator} {bound}")]
    #[backtrace(private_constructor)]
    OutOfRange {
        value: String,
        comparator: &'static str,
        bound: String,
    },

    #[error("length of {ty} out of range: {length} {comparator} {bound}")]
    #[backtrace(private_constructor)]
    LengthOutOfRange {
        ty: &'static str,
        length: usize,
        comparator: &'static str,
        bound: usize,
    },

    #[error("out of gas! limit: {limit}, used: {used}, comment: {comment}")]
    OutOfGas {
        limit: u64,
        used: u64,
        comment: &'static str,
    },

    #[error("failed to serialize! codec: {codec}, type: {ty}, reason: {reason}")]
    #[backtrace(private_constructor)]
    Serialize {
        codec: &'static str,
        ty: &'static str,
        reason: String,
    },

    #[error("failed to deserialize! codec: {codec}, type: {ty}, reason: {reason}")]
    #[backtrace(private_constructor)]
    Deserialize {
        codec: &'static str,
        ty: &'static str,
        reason: String,
    },
}

impl StdError {
    pub fn host(msg: String) -> Self {
        msg.into()
    }

    pub fn invalid_denom<D>(denom: D, reason: &'static str) -> Self
    where
        D: ToString,
    {
        Self::_invalid_denom(denom.to_string(), reason)
    }

    pub fn invalid_coins<R>(reason: R) -> Self
    where
        R: ToString,
    {
        Self::_invalid_coins(reason.to_string())
    }

    pub fn invalid_payment<A, B>(expect: A, actual: B) -> Self
    where
        A: ToString,
        B: ToString,
    {
        Self::_invalid_payment(expect.to_string(), actual.to_string())
    }

    pub fn data_not_found<T>(key: &[u8]) -> Self {
        Self::_data_not_found(type_name::<T>(), BASE64.encode(key))
    }

    pub fn duplicate_data<T>() -> Self {
        Self::_duplicate_data(type_name::<T>())
    }

    pub fn empty_value<T>() -> Self {
        Self::_empty_value(type_name::<T>())
    }

    pub fn zero_value<T>() -> Self {
        Self::_zero_value(type_name::<T>())
    }

    pub fn out_of_range<T>(value: T, comparator: &'static str, bound: T) -> Self
    where
        T: ToString,
    {
        Self::_out_of_range(value.to_string(), comparator, bound.to_string())
    }

    pub fn length_out_of_range<T>(length: usize, comparator: &'static str, bound: usize) -> Self {
        Self::_length_out_of_range(type_name::<T>(), length, comparator, bound)
    }

    pub fn serialize<T, R>(codec: &'static str, reason: R) -> Self
    where
        R: ToString,
    {
        Self::_serialize(codec, type_name::<T>(), reason.to_string())
    }

    pub fn deserialize<T, R>(codec: &'static str, reason: R) -> Self
    where
        R: ToString,
    {
        Self::_deserialize(codec, type_name::<T>(), reason.to_string())
    }
}

pub type StdResult<T> = core::result::Result<T, StdError>;
