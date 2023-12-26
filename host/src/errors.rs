use wasmi::{core::HostError, errors::MemoryError};

// we can't use anyhow::Error, because it doesn't implement wasi::core::HostError
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Wasmi(#[from] wasmi::Error),

    #[error("Can't find the desired Wasm export")]
    ExportNotFound,

    #[error("Export is not a function")]
    ExportIsNotFunc,

    #[error("Export is not a memory")]
    ExportIsNotMemory,

    #[error("Failed to parse Region: expect 12 bytes, found {0}")]
    ParseRegion(usize),

    #[error("Region too small! capacity: {capacity}, attempting to write: {length}")]
    InsufficientRegion {
        capacity: u32,
        length:   u32,
    },

    #[error("Failed to read memory! offset: {offset}, length: {length}, reason: {reason}")]
    ReadMemory {
        offset: usize,
        length: usize,
        reason: MemoryError,
    },

    #[error(
        "Failed to write to Wasm memory! offset: {offset}, length: {length}, reason: {reason}"
    )]
    WriteMemory {
        offset: usize,
        length: usize,
        reason: MemoryError,
    },
}

// important
impl HostError for Error {}
