use {
    crate::{Event, StdError, StdResult},
    serde::{Deserialize, Serialize},
};

/// The result for executing a submessage, provided to the contract in the `reply`
/// entry point.
pub type SubMsgResult = GenericResult<Vec<Event>>;

/// A result type that can be serialized into a string and thus passed over the
/// FFI boundary.
///
/// This is used in two cases:
/// - the host calls an export function on the Wasm module
/// - the Wasm module calls an import function provided by the host
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GenericResult<T> {
    Ok(T),
    Err(String),
}

impl<T, E> From<Result<T, E>> for GenericResult<T>
where
    E: ToString,
{
    fn from(res: Result<T, E>) -> Self {
        match res {
            Result::Ok(data) => Self::Ok(data),
            Result::Err(err) => Self::Err(err.to_string()),
        }
    }
}

impl<T> GenericResult<T> {
    /// Convert the GenericResult to a StdResult, so that it can be unwrapped
    /// with the `?` operator.
    pub fn into_std_result(self) -> StdResult<T> {
        match self {
            GenericResult::Ok(data) => Ok(data),
            GenericResult::Err(err) => Err(StdError::Generic(err)),
        }
    }

    /// Assume the GenericResult is an Ok, get the data it carries. Error if it
    /// is an Err.
    /// This is useful if you're sure the result is an Ok, e.g. when handling a
    /// submessage result in the `reply` entry point, when you have configured
    /// it to reply only on success.
    pub fn as_ok(self) -> T {
        match self {
            GenericResult::Ok(data) => data,
            GenericResult::Err(_) => unreachable!(),
        }
    }

    /// Assume the GenericResult is an Err, get the error message. Error if it
    /// is an Ok.
    /// This is useful if you're sure the result is an Err, e.g. when handling a
    /// submessage result in the `reply` entry point, when you have configured
    /// it to reply only on error.
    pub fn as_err(self) -> String {
        match self {
            GenericResult::Ok(_) => unreachable!(),
            GenericResult::Err(err) => err,
        }
    }
}
