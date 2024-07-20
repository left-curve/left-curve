use {grug_app::VmError, grug_types::StdError, thiserror::Error};

#[derive(Debug, Error)]
pub enum RustVmError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error("attempting to call `{name}` with {num} inputs, but this function takes a different number of inputs")]
    IncorrectNumberOfInputs { name: String, num: usize },

    #[error(transparent)]
    VmError(#[from] VmError),
}

impl From<RustVmError> for VmError {
    fn from(err: RustVmError) -> Self {
        match err {
            RustVmError::VmError(vm_error) => vm_error,
            _ => VmError::GenericError(err.to_string()),
        }
    }
}

pub type RustVmResult<T> = core::result::Result<T, RustVmError>;
