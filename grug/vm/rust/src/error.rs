use {
    grug_app::AppError,
    grug_types::{Backtraceable, StdError},
};

#[grug_macros::backtrace]
pub enum VmError {
    #[error(transparent)]
    Std(StdError),

    #[error("contract with index `{index}` not found")]
    #[backtrace(private_constructor)]
    ContractNotFound { index: usize },

    #[error("contract does not implement function `{name}`")]
    #[backtrace(private_constructor)]
    FunctionNotFound { name: &'static str },

    #[error("unknown function: `{name}`")]
    #[backtrace(private_constructor)]
    UnknownFunction { name: &'static str },

    #[error(
        "attempting to call `{name}` with {num} inputs, but this function takes a different number of inputs"
    )]
    #[backtrace(private_constructor)]
    IncorrectNumberOfInputs { name: &'static str, num: usize },
}

impl VmError {
    pub fn contract_not_found(index: usize) -> Self {
        Self::_contract_not_found(index)
    }

    pub fn function_not_found(name: &'static str) -> Self {
        Self::_function_not_found(name)
    }

    pub fn unknown_function(name: &'static str) -> Self {
        Self::_unknown_function(name)
    }

    pub fn incorrect_number_of_inputs(name: &'static str, num: usize) -> Self {
        Self::_incorrect_number_of_inputs(name, num)
    }
}

impl From<VmError> for AppError {
    fn from(err: VmError) -> Self {
        let err = err.into_generic_backtraced_error();
        AppError::Vm {
            error: err.to_string(),
            backtrace: err.backtrace(),
        }
    }
}

pub type VmResult<T> = core::result::Result<T, VmError>;
