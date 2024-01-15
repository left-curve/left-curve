use {
    thiserror::Error,
    cw_std::StdError,
    std::string::FromUtf8Error,
    wasmer::{CompileError, ExportError, InstantiationError, MemoryAccessError, RuntimeError},
};

#[derive(Debug, Error)]
pub enum VmError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error(transparent)]
    Compile(#[from] CompileError),

    #[error(transparent)]
    Export(#[from] ExportError),

    #[error(transparent)]
    Instantiation(#[from] InstantiationError),

    #[error(transparent)]
    MemoryAccess(#[from] MemoryAccessError),

    #[error(transparent)]
    Runtime(#[from] RuntimeError),

    #[error(transparent)]
    FromUtf8(#[from] FromUtf8Error),

    #[error("Memory is not set in Environment")]
    MemoryNotSet,

    #[error("Store is not set in ContextData")]
    StoreNotSet,

    #[error("Wasmer instance is not set in ContextData")]
    WasmerInstanceNotSet,

    #[error("Failed to read lock ContextData")]
    FailedReadLock,

    #[error("Failed to write lock ContextData")]
    FailedWriteLock,

    #[error("Region is too small! offset: {offset}, capacity: {capacity}, data: {data}")]
    RegionTooSmall {
        offset:   u32,
        capacity: u32,
        data:     String,
    },

    #[error("Unexpected number of return values! name: {name}, expect: {expect}, actual: {actual}")]
    ReturnCount {
        name:   String,
        expect: usize,
        actual: usize,
    },

    #[error("Unexpected return type: {0}")]
    ReturnType(&'static str),

    #[error("Cannot find iterator with id `{iterator_id}`")]
    IteratorNotFound {
        iterator_id: i32,
    },
}

// required such that VmError can be used in import function signatures
impl From<VmError> for RuntimeError {
    fn from(vm_err: VmError) -> Self {
        RuntimeError::new(vm_err.to_string())
    }
}

pub type VmResult<T> = std::result::Result<T, VmError>;
