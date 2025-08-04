use {
    crate::Event,
    borsh::{BorshDeserialize, BorshSerialize},
    grug_types_base::{Backtraceable, BacktracedError},
    std::ops::Deref,
};

/// Result of which the error is a string.
///
/// If a result is passed through the FFI boundary, it is received on the other
/// side as a generic result.
///
/// This is used in two cases:
///
/// - the host calls an export function on the Wasm module (result is passed
///   from the module to the host);
/// - the Wasm module calls an import function provided by the host (result is
///   passed from the host to the module).
pub type GenericResult<T> = Result<T, BacktracedError<String>>;

pub type QueryResult<T> = Result<T, String>;

/// The result for executing a submessage.
///
/// This is provided to the contract in the `reply` entry point.
#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct SubMsgResult(Result<Event, String>);

impl SubMsgResult {
    pub fn ok(event: Event) -> Self {
        Self(Ok(event))
    }

    pub fn err<E>(error: &E) -> Self
    where
        E: Backtraceable,
    {
        Self(Err(error.error()))
    }

    pub fn into_result(self) -> Result<Event, String> {
        self.0
    }
}

impl Deref for SubMsgResult {
    type Target = Result<Event, String>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Describes an error of which the error can be stringified, and thus, can be
/// passed across the FFI boundary.
pub trait GenericResultExt<T> {
    fn into_generic_result(self) -> GenericResult<T>;
}

impl<T, E> GenericResultExt<T> for Result<T, E>
where
    E: Backtraceable,
{
    fn into_generic_result(self) -> GenericResult<T> {
        self.map_err(Backtraceable::into_generic_backtraced_error)
    }
}
