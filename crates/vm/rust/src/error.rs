use {
    grug_app::{AppError, VmError},
    grug_types::StdError,
    thiserror::Error,
};

#[derive(Debug, Error)]
pub enum RustVmError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error("attempting to call `{name}` with {num} inputs, but this function takes a different number of inputs")]
    IncorrectNumberOfInputs { name: String, num: usize },

    #[error(transparent)]
    VmError(#[from] VmError),
}

impl From<RustVmError> for AppError {
    fn from(err: RustVmError) -> Self {
        match err {
            RustVmError::VmError(vm_error) => AppError::VM(vm_error),
            _ => AppError::VM(VmError::GenericError(err.to_string())),
        }
    }
}

pub type VmResult<T> = core::result::Result<T, RustVmError>;
