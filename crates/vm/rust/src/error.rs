use {grug_app::AppError, grug_types::StdError, thiserror::Error};

#[derive(Debug, Error)]
pub enum VmError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error("attempting to call `{name}` with {num} inputs, but this function takes a different number of inputs")]
    IncorrectNumberOfInputs { name: &'static str, num: usize },
}

impl From<VmError> for AppError {
    fn from(err: VmError) -> Self {
        AppError::Vm(err.to_string())
    }
}

pub type VmResult<T> = core::result::Result<T, VmError>;
