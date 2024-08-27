use {
    crate::{Event, StdError, StdResult},
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
    /// Convert the `GenericResult<T>` to an `StdResult<T>`, so that it can be
    /// unwrapped with the `?` operator.
    pub fn into_std_result(self) -> StdResult<T> {
        match self {
            GenericResult::Ok(data) => Ok(data),
            GenericResult::Err(err) => Err(StdError::Generic(err)),
        }
    }

    /// Convert the `GenericResult<T>` to an `Option<T>`, discarding the error
    /// message.
    pub fn ok(self) -> Option<T> {
        match self {
            GenericResult::Ok(data) => Some(data),
            GenericResult::Err(_) => None,
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
            GenericResult::Ok(value) => assert_eq!(value, expect, "wrong value!"),
            GenericResult::Err(err) => panic!("expecting ok, got error: {err}"),
        }
    }

    /// Ensure the result is ok, but the value doesn't equal the given value.
    pub fn should_succeed_but_not_equal<U>(self, expect: U)
    where
        T: Debug + PartialEq<U>,
        U: Debug,
    {
        match self {
            GenericResult::Ok(value) => assert_ne!(value, expect, "wrong value!"),
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

    /// Ensure the result matches the given result.
    pub fn should_match<U>(self, expect: GenericResult<U>)
    where
        T: Debug + PartialEq<U>,
        U: Debug,
    {
        match (self, expect) {
            (GenericResult::Ok(actual), GenericResult::Ok(expect)) => {
                assert_eq!(actual, expect, "wrong value!");
            },
            (GenericResult::Err(actual), GenericResult::Err(expect)) => {
                assert!(
                    actual.contains(&expect),
                    "wrong error! expecting: {expect}, got {actual}"
                );
            },
            (GenericResult::Ok(value), GenericResult::Err(_)) => {
                panic!("expecting error, got ok: {value:?}");
            },
            (GenericResult::Err(err), GenericResult::Ok(_)) => {
                panic!("expecting ok, got error: {err}");
            },
        }
    }
}
