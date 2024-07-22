use {
    crate::{Event, StdError, StdResult},
    borsh::{BorshDeserialize, BorshSerialize},
    serde::{Deserialize, Serialize},
    std::fmt::Debug,
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
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
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
    pub fn map<U, F>(&mut self, op: F)
    where
        F: FnOnce(&mut T) -> U,
    {
        if let GenericResult::Ok(data) = self {
            op(data);
        }
    }

    pub fn map_err<F, O>(&mut self, op: O)
    where
        O: FnOnce(&mut String) -> String,
    {
        if let GenericResult::Err(err) = self {
            op(err);
        }
    }

    pub fn ok(self) -> Option<T> {
        match self {
            GenericResult::Ok(data) => Some(data),
            GenericResult::Err(_) => None,
        }
    }

    /// Convert the `GenericResult<T>` to an `StdResult<T>`, so that it can be
    /// unwrapped with the `?` operator.
    pub fn into_std_result(self) -> StdResult<T> {
        match self {
            GenericResult::Ok(data) => Ok(data),
            GenericResult::Err(err) => Err(StdError::Generic(err)),
        }
    }

    /// Ensure the result is ok; return the value.
    pub fn should_succeed(self) -> T {
        match self {
            GenericResult::Ok(value) => value,
            GenericResult::Err(err) => panic!("expecting ok, got error: {err}"),
        }
    }

    /// Ensure the result is error; return the error message;
    pub fn should_fail(self) -> String
    where
        T: Debug,
    {
        match self {
            GenericResult::Err(err) => err,
            GenericResult::Ok(value) => panic!("expecting error, got ok: {value:?}"),
        }
    }

    /// Ensure the result is ok, and matches the expect value.
    pub fn should_succeed_and_equal<U>(self, expect: U)
    where
        T: Debug + PartialEq<U>,
        U: Debug,
    {
        match self {
            GenericResult::Ok(value) => {
                assert_eq!(
                    value, expect,
                    "wrong value! expecting: {expect:?}, got: {value:?}"
                );
            },
            GenericResult::Err(err) => panic!("expecting ok, got error: {err}"),
        }
    }

    /// Ensure the result is error, and contains the given message.
    pub fn should_fail_with_error<M>(self, msg: M)
    where
        T: Debug,
        M: ToString,
    {
        match self {
            GenericResult::Err(err) => {
                // Here we stringify the error and check for the existence of
                // the substring, instead of utilizing the Rust type system.
                //
                // Have to go with this approach because errors emitted by the
                // contract are converted to strings (as `GenericResult`) when
                // passed through the FFI, at which time they lost their types.
                let expect = msg.to_string();
                let actual = err.to_string();
                assert!(
                    actual.contains(&expect),
                    "wrong error! expecting: {expect}, got: {actual}"
                );
            },
            GenericResult::Ok(value) => panic!("expecting error, got ok: {value:?}"),
        }
    }
}
