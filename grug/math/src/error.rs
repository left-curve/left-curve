use {grug_backtrace::BT, std::any::type_name};

#[grug_macros::backtrace(grug_backtrace)]
#[derive(Clone)]
pub enum MathError {
    #[error("failed to parse string `{input}` into {ty}: {reason}")]
    #[backtrace(private_constructor)]
    ParseNumber {
        ty: &'static str,
        input: String,
        reason: String,
    },

    #[error("conversion overflow: {source_type}({value}) > {target_type}::MAX")]
    #[backtrace(private_constructor)]
    OverflowConversion {
        source_type: &'static str,
        target_type: &'static str,
        value: String,
    },

    #[error("addition overflow: {a} + {b} (type: {ty})")]
    #[backtrace(private_constructor)]
    OverflowAdd {
        ty: &'static str,
        a: String,
        b: String,
    },

    #[error("subtraction overflow: {a} - {b} (type: {ty})")]
    #[backtrace(private_constructor)]
    OverflowSub {
        ty: &'static str,
        a: String,
        b: String,
    },

    #[error("multiplication overflow: {a} (type: {t1}) * {b} (type: {t2})")]
    #[backtrace(private_constructor)]
    OverflowMul {
        t1: &'static str,
        t2: &'static str,
        a: String,
        b: String,
    },

    #[error("power overflow: {a} ^ {b} (type: {ty})")]
    #[backtrace(private_constructor)]
    OverflowPow {
        ty: &'static str,
        a: String,
        b: String,
    },

    #[error("left shift overflow: {a} << {b}")]
    #[backtrace(private_constructor)]
    OverflowShl { a: String, b: u32 },

    #[error("right shift overflow: {a} >> {b}")]
    #[backtrace(private_constructor)]
    OverflowShr { a: String, b: u32 },

    #[error("division by zero: {a} / 0")]
    #[backtrace(private_constructor)]
    DivisionByZero { a: String },

    #[error("remainder by zero: {a} % 0")]
    #[backtrace(private_constructor)]
    RemainderByZero { a: String },

    #[error("multiply a non-negative lhs with a negative rhs: {ty}({a}) * {b}")]
    #[backtrace(private_constructor)]
    NegativeMul {
        ty: &'static str,
        a: String,
        b: String,
    },

    #[error("divide a non-negative lhs with a negative rhs: {ty}({a}) / {b}")]
    #[backtrace(private_constructor)]
    NegativeDiv {
        ty: &'static str,
        a: String,
        b: String,
    },

    #[error("square root of negative: sqrt({a})")]
    #[backtrace(private_constructor)]
    NegativeSqrt { a: String },

    #[error("square root failed")]
    SqrtFailed,

    #[error("logarithm of zero")]
    ZeroLog,
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
            backtrace: BT::default(),
        }
    }

    pub fn overflow_conversion<A, B>(source: A) -> Self
    where
        A: ToString,
    {
        Self::_overflow_conversion(type_name::<A>(), type_name::<B>(), source.to_string())
    }

    pub fn overflow_add<T>(a: T, b: T) -> Self
    where
        T: ToString,
    {
        Self::_overflow_add(type_name::<T>(), a.to_string(), b.to_string())
    }

    pub fn overflow_sub<T>(a: T, b: T) -> Self
    where
        T: ToString,
    {
        Self::_overflow_sub(type_name::<T>(), a.to_string(), b.to_string())
    }

    pub fn overflow_mul<T1, T2>(a: T1, b: T2) -> Self
    where
        T1: ToString,
        T2: ToString,
    {
        Self::_overflow_mul(
            type_name::<T1>(),
            type_name::<T2>(),
            a.to_string(),
            b.to_string(),
        )
    }

    pub fn overflow_pow<T>(a: T, b: u32) -> Self
    where
        T: ToString,
    {
        Self::_overflow_pow(type_name::<T>(), a.to_string(), b.to_string())
    }

    pub fn overflow_shl<T>(a: T, b: u32) -> Self
    where
        T: ToString,
    {
        Self::_overflow_shl(a.to_string(), b)
    }

    pub fn overflow_shr<T>(a: T, b: u32) -> Self
    where
        T: ToString,
    {
        Self::_overflow_shr(a.to_string(), b)
    }

    pub fn division_by_zero<T>(a: T) -> Self
    where
        T: ToString,
    {
        Self::_division_by_zero(a.to_string())
    }

    pub fn remainder_by_zero<T>(a: T) -> Self
    where
        T: ToString,
    {
        Self::_remainder_by_zero(a.to_string())
    }

    pub fn negative_mul<A, B>(a: A, b: B) -> Self
    where
        A: ToString,
        B: ToString,
    {
        Self::_negative_mul(type_name::<A>(), a.to_string(), b.to_string())
    }

    pub fn negative_div<A, B>(a: A, b: B) -> Self
    where
        A: ToString,
        B: ToString,
    {
        Self::_negative_div(type_name::<A>(), a.to_string(), b.to_string())
    }

    pub fn negative_sqrt<T>(a: T) -> Self
    where
        T: ToString,
    {
        Self::_negative_sqrt(a.to_string())
    }
}

pub type MathResult<T> = Result<T, MathError>;
