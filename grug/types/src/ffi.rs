use std::fmt::Debug;

use crate::Event;

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
pub type GenericResult<T> = Result<T, String>;

/// The result for executing a submessage.
///
/// This is provided to the contract in the `reply` entry point.
pub type SubMsgResult = GenericResult<Event>;

/// Describes an error of which the error can be stringified, and thus, can be
/// passed across the FFI boundary.
pub trait GenericResultExt<T> {
    fn into_generic_result(self) -> GenericResult<T>;
}

impl<T, E> GenericResultExt<T> for Result<T, E>
where
    E: Debug,
{
    fn into_generic_result(self) -> GenericResult<T> {
        self.map_err(|e| format!("{e:?}"))
    }
}
