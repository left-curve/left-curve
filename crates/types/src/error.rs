use {
    data_encoding::BASE64,
    hex::FromHexError,
    std::{any::type_name, array::TryFromSliceError},
    thiserror::Error,
};

#[derive(Debug, Error)]
pub enum StdError {
    #[error(transparent)]
    FromHex(#[from] FromHexError),

    #[error(transparent)]
    TryFromSlice(#[from] TryFromSliceError),

    #[error("Generic error: {0}")]
    Generic(String),

    // TODO: add more details to this
    #[error("Signature verification failed")]
    VerificationFailed,

    #[error("Host failed to specify `{field}` in context")]
    MissingContext {
        field: &'static str,
    },

    #[error("Failed to parse string `{value}` into {ty}: {reason}")]
    ParseNumber {
        ty:     &'static str,
        value:  String,
        reason: String,
    },

    #[error("Failed to parse into Coins: {reason}")]
    ParseCoins {
        reason: String,
    },

    #[error("Invalid payment: expecting {expect} coins, found {actual}")]
    Payment {
        expect: usize,
        actual: usize,
    },

    #[error("Cannot find denom `{denom}` in coins")]
    DenomNotFound {
        denom: String,
    },

    #[error("Data not found! type: {ty}, storage key: {key}")]
    DataNotFound {
        ty:  &'static str,
        key: String,
    },

    #[error("Cannot find iterator with ID {iterator_id}")]
    IteratorNotFound {
        iterator_id: i32,
    },

    #[error("Conversion overflow: {source_type}({value}) > {target_type}::MAX")]
    OverflowConversion {
        source_type: &'static str,
        target_type: &'static str,
        value:       String,
    },

    #[error("Addition overflow: {a} + {b} > {ty}::MAX")]
    OverflowAdd {
        ty: &'static str,
        a:  String,
        b:  String,
    },

    #[error("Subtraction overflow: {a} - {b} < {ty}::MIN")]
    OverflowSub {
        ty: &'static str,
        a:  String,
        b:  String,
    },

    #[error("Multiplication overflow: {a} * {b} > {ty}::MAX")]
    OverflowMul {
        ty: &'static str,
        a:  String,
        b:  String,
    },

    #[error("Power overflow: {a} ^ {b} > {ty}::MAX")]
    OverflowPow {
        ty: &'static str,
        a:  String,
        b:  String,
    },

    #[error("Left shift overflow: {a} << {b}")]
    OverflowShl {
        a:  String,
        b:  u32,
    },

    #[error("Right shift overflow: {a} >> {b}")]
    OverflowShr {
        a:  String,
        b:  u32,
    },

    #[error("Division by zero: {a} / 0")]
    DivisionByZero {
        a: String,
    },

    #[error("Remainder by zero: {a} % 0")]
    RemainderByZero {
        a: String,
    },

    #[error("Failed to serialize into json! type: {ty}, reason: {reason}")]
    Serialize {
        ty:     &'static str,
        reason: String,
    },

    #[error("Failed to deserialize from json! type: {ty}, reason: {reason}")]
    Deserialize {
        ty:     &'static str,
        reason: String,
    },
}

impl StdError {
    pub const fn missing_context(field: &'static str) -> Self {
        Self::MissingContext {
            field,
        }
    }

    pub fn parse_number<A>(value: impl ToString, reason: impl ToString) -> Self {
        Self::ParseNumber {
            ty:     type_name::<A>(),
            value:  value.to_string(),
            reason: reason.to_string(),
        }
    }

    pub fn parse_coins(reason: impl Into<String>) -> Self {
        Self::ParseCoins {
            reason: reason.into(),
        }
    }

    pub fn payment(expect: usize, actual: usize) -> Self {
        Self::Payment {
            expect,
            actual,
        }
    }

    pub fn data_not_found<T>(key: &[u8]) -> Self {
        Self::DataNotFound {
            ty:  type_name::<T>(),
            key: BASE64.encode(key),
        }
    }

    pub fn overflow_conversion<A: ToString, B>(source: A) -> Self {
        Self::OverflowConversion {
            source_type: type_name::<A>(),
            target_type: type_name::<B>(),
            value:       source.to_string(),
        }
    }

    pub fn overflow_add<T: ToString>(a: T, b: T) -> Self {
        Self::OverflowAdd {
            ty: type_name::<T>(),
            a:  a.to_string(),
            b:  b.to_string(),
        }
    }

    pub fn overflow_sub<T: ToString>(a: T, b: T) -> Self {
        Self::OverflowSub {
            ty: type_name::<T>(),
            a:  a.to_string(),
            b:  b.to_string(),
        }
    }

    pub fn overflow_mul<T: ToString>(a: T, b: T) -> Self {
        Self::OverflowMul {
            ty: type_name::<T>(),
            a:  a.to_string(),
            b:  b.to_string(),
        }
    }

    pub fn overflow_pow<T: ToString>(a: T, b: u32) -> Self {
        Self::OverflowPow {
            ty: type_name::<T>(),
            a:  a.to_string(),
            b:  b.to_string(),
        }
    }

    pub fn overflow_shl<T: ToString>(a: T, b: u32) -> Self {
        Self::OverflowShl {
            a: a.to_string(),
            b,
        }
    }

    pub fn overflow_shr<T: ToString>(a: T, b: u32) -> Self {
        Self::OverflowShr {
            a: a.to_string(),
            b,
        }
    }

    pub fn division_by_zero<T: ToString>(a: T) -> Self {
        Self::DivisionByZero {
            a: a.to_string(),
        }
    }

    pub fn remainder_by_zero<T: ToString>(a: T) -> Self {
        Self::RemainderByZero {
            a: a.to_string(),
        }
    }

    pub fn serialize<T>(reason: impl ToString) -> Self {
        Self::Serialize {
            ty:     type_name::<T>(),
            reason: reason.to_string(),
        }
    }

    pub fn deserialize<T>(reason: impl ToString) -> Self {
        Self::Deserialize {
            ty:     type_name::<T>(),
            reason: reason.to_string(),
        }
    }
}

pub type StdResult<T> = std::result::Result<T, StdError>;
