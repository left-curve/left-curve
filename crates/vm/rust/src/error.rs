use {cw_app::AppError, cw_types::StdError, thiserror::Error};

#[derive(Debug, Error)]
pub enum VmError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error("attempting to call `{name}` with {num} inputs, but this function takes a different number of inputs")]
    IncorrectNumberOfInputs {
        name: String,
        num: usize,
    },
}

impl From<VmError> for AppError {
    fn from(err: VmError) -> Self {
        AppError::Vm(err.to_string())
    }
}

pub type VmResult<T> = std::result::Result<T, VmError>;
