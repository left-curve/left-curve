use {grug_app::AppError, grug_types::StdError, thiserror::Error};

#[derive(Debug, Error)]
pub enum VmError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error("attempting to call `{name}` with {num} inputs, but this function takes a different number of inputs")]
    IncorrectNumberOfInputs { name: &'static str, num: usize },

    #[error("contract does not implement function `{name}`")]
    FunctionNotFound { name: &'static str },
}

impl VmError {
    pub const fn function_not_found(name: &'static str) -> Self {
        Self::FunctionNotFound { name }
    }
}

impl From<VmError> for AppError {
    fn from(err: VmError) -> Self {
        AppError::Vm(err.to_string())
    }
}

pub type VmResult<T> = core::result::Result<T, VmError>;
