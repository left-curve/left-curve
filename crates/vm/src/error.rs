use {
    thiserror::Error,
    cw_db::DbError,
    cw_std::StdError,
    std::string::FromUtf8Error,
    wasmer::{CompileError, ExportError, InstantiationError, MemoryAccessError, RuntimeError},
};

#[derive(Debug, Error)]
pub enum VmError {
    #[error(transparent)]
    Db(#[from] DbError),

    #[error(transparent)]
    Std(#[from] StdError),

    #[error(transparent)]
    FromUtf8(#[from] FromUtf8Error),

    #[error(transparent)]
    Export(#[from] ExportError),

    #[error(transparent)]
    MemoryAccess(#[from] MemoryAccessError),

    #[error(transparent)]
    Runtime(#[from] RuntimeError),

    // the wasmer CompileError and InstantiateError are big (56 and 128 bytes,
    // respectively). we get a clippy warning if we wrap them directly here in
    // VmError (result_large_err). to avoid this we cast them to strings instead.
    #[error("Failed to instantiate Wasm module: {0}")]
    Instantiation(String),

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
}

impl From<CompileError> for VmError {
    fn from(err: CompileError) -> Self {
        Self::Instantiation(err.to_string())
    }
}

impl From<InstantiationError> for VmError {
    fn from(err: InstantiationError) -> Self {
        Self::Instantiation(err.to_string())
    }
}

// required such that VmError can be used in import function signatures
impl From<VmError> for RuntimeError {
    fn from(vm_err: VmError) -> Self {
        RuntimeError::new(vm_err.to_string())
    }
}

pub type VmResult<T> = std::result::Result<T, VmError>;
