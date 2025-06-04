use {std::any::type_name, thiserror::Error};

#[derive(Debug, Clone, Error)]
pub enum MathError {
    #[error("failed to parse string `{input}` into {ty}: {reason}")]
    ParseNumber {
        ty: &'static str,
        input: String,
        reason: String,
    },

    #[error("conversion overflow: {source_type}({value}) > {target_type}::MAX")]
    OverflowConversion {
        source_type: &'static str,
        target_type: &'static str,
        value: String,
    },

    #[error("addition overflow: {a} + {b} (type: {ty})")]
    OverflowAdd {
        ty: &'static str,
        a: String,
        b: String,
    },

    #[error("subtraction overflow: {a} - {b} (type: {ty})")]
    OverflowSub {
        ty: &'static str,
        a: String,
        b: String,
    },

    #[error("multiplication overflow: {a} * {b} (type: {ty})")]
    OverflowMul {
        ty: &'static str,
        a: String,
        b: String,
    },

    #[error("power overflow: {a} ^ {b} (type: {ty})")]
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

    #[error("square root failed")]
    SqrtFailed,

    #[error("logarithm of zero")]
    ZeroLog,

    #[error("invalid negation. Can only negate signed types or unsigned types with a zero value")]
    InvalidNegation,

    #[error("overflow when negating {ty}({value})")]
    OverflowNegation { ty: &'static str, value: String },
}

impl MathError {
    pub fn parse_number<T, V, R>(input: V, reason: R) -> Self
    where
        V: ToString,
        R: ToString,
    {
        Self::ParseNumber {
            ty: type_name::<T>(),
            input: input.to_string(),
            reason: reason.to_string(),
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

    pub fn zero_log() -> Self {
        Self::ZeroLog
    }
}

pub type MathResult<T> = Result<T, MathError>;
