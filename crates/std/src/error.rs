use {
    data_encoding::BASE64,
    hex::FromHexError,
    std::{any::type_name, num::ParseIntError},
    thiserror::Error,
};

#[derive(Debug, Error)]
pub enum StdError {
    #[error(transparent)]
    FromHex(#[from] FromHexError),

    #[error(transparent)]
    ParseInt(#[from] ParseIntError),

    #[error("Generic error: {0}")]
    Generic(String),

    // TODO: add more details to this
    #[error("Signature verification failed")]
    VerificationFailed,

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

    #[error("Division by zero: {a} / 0")]
    DivisionByZero {
        a: String,
    },

    // #[error("Incorrect length! type: {ty}, expected: {expect}, actual: {actual}")]
    // IncorrectLength {
    //     ty:     &'static str,
    //     expect: usize,
    //     actual: usize,
    // },

    // #[error("String does not start with the expected prefix: {prefix}")]
    // IncorrectPrefix {
    //     ty:     &'static str,
    //     prefix: String,
    // },

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

    pub fn division_by_zero<T: ToString>(a: T) -> Self {
        Self::DivisionByZero {
            a: a.to_string(),
        }
    }

    // pub fn incorrect_length<T>(expect: usize, actual: usize) -> Self {
    //     Self::IncorrectLength {
    //         ty: type_name::<T>(),
    //         expect,
    //         actual,
    //     }
    // }

    // pub fn incorrect_prefix<T>(prefix: impl ToString) -> Self {
    //     Self::IncorrectPrefix {
    //         ty:     type_name::<T>(),
    //         prefix: prefix.to_string(),
    //     }
    // }

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
