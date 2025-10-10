use {
    grug_app::AppError,
    grug_types::{Backtraceable, StdError},
    std::string::FromUtf8Error,
    wasmer::{CompileError, ExportError, InstantiationError, MemoryAccessError, RuntimeError},
};

// #[derive(Debug, Error)]
#[grug_macros::backtrace]
pub enum VmError {
    #[error(transparent)]
    Std(StdError),

    #[error(transparent)]
    #[backtrace(new)]
    FromUtf8(FromUtf8Error),

    #[error(transparent)]
    #[backtrace(new)]
    Export(ExportError),

    #[error(transparent)]
    #[backtrace(new)]
    MemoryAccess(MemoryAccessError),

    #[error(transparent)]
    #[backtrace(new)]
    Runtime(RuntimeError),

    // The wasmer `CompileError` and `InstantiateError` are big (56 and 128 bytes,
    // respectively). We get a clippy warning if we wrap them directly here in
    // VmError (result_large_err). To avoid this, we cast them to strings instead.
    #[error("failed to instantiate Wasm module: {message}")]
    Instantiation { message: String },

    #[error("Wasmer memory not set in Environment")]
    WasmerMemoryNotSet,

    #[error("Wasmer memory already set in Environment")]
    WasmerMemoryAlreadySet,

    #[error("Wasmer instance not set in ContextData")]
    WasmerInstanceNotSet,

    #[error("Wasmer instance already set in ContextData")]
    WasmerInstanceAlreadySet,

    #[error("iterator with ID `{iterator_id}` not found")]
    IteratorNotFound { iterator_id: i32 },

    #[error("region has a 0 offset")]
    RegionZeroOffset,

    #[error("region length exceeds capacity! length: {length}, capacity: {capacity}")]
    RegionLengthExceedsCapacity { length: u32, capacity: u32 },

    #[error("region exceeds address space! offset: {offset}, capacity: {capacity}")]
    RegionOutOfRange { offset: u32, capacity: u32 },

    #[error("region is too small! offset: {offset}, capacity: {capacity}, data: {data}")]
    RegionTooSmall {
        offset: u32,
        capacity: u32,
        data: String,
    },

    #[error("unexpected return value count! name: {name}, expect: {expect}, actual: {actual}")]
    ReturnCount {
        name: String,
        expect: usize,
        actual: usize,
    },

    #[error("unexpected return type: {message}")]
    ReturnType { message: &'static str },

    #[error("attempt to write to storage during an state immutable call")]
    ImmutableState,

    #[error("max query depth exceeded")]
    ExceedMaxQueryDepth,
}

impl From<CompileError> for VmError {
    fn from(err: CompileError) -> Self {
        Self::instantiation(err.to_string())
    }
}

impl From<InstantiationError> for VmError {
    fn from(err: InstantiationError) -> Self {
        Self::instantiation(err.to_string())
    }
}

// required such that VmError can be used in import function signatures
impl From<VmError> for RuntimeError {
    fn from(err: VmError) -> Self {
        RuntimeError::new(err.to_string())
    }
}

impl From<VmError> for AppError {
    fn from(err: VmError) -> Self {
        let err = err.into_generic_backtraced_error();
        AppError::Vm {
            error: err.error,
            backtrace: err.backtrace,
        }
    }
}

pub type VmResult<T> = core::result::Result<T, VmError>;
