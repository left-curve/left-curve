use {
    crate::{Event, Outcome, StdError, StdResult, TxError, TxOutcome, TxSuccess},
    borsh::{BorshDeserialize, BorshSerialize},
    serde::{Deserialize, Serialize},
    std::fmt::{Debug, Display},
};

/// The result for executing a submessage, provided to the contract in the `reply`
/// entry point.
pub type SubMsgResult = GenericResult<Vec<Event>>;

// ------------------------------ generic result -------------------------------

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

    pub fn err(self) -> Option<String> {
        match self {
            GenericResult::Ok(_) => None,
            GenericResult::Err(err) => Some(err),
        }
    }

    pub fn is_ok(&self) -> bool {
        matches!(self, GenericResult::Ok(_))
    }

    pub fn is_err(&self) -> bool {
        matches!(self, GenericResult::Err(_))
    }
}

impl<T, E> From<Result<T, E>> for GenericResult<T>
where
    E: ToString,
{
    fn from(result: Result<T, E>) -> Self {
        match result {
            Ok(value) => GenericResult::Ok(value),
            Err(err) => GenericResult::Err(err.to_string()),
        }
    }
}

// ------------------------------ extension trait ------------------------------

/// Addition methods for result types.
/// Useful for testing, improving code readability.
pub trait ResultExt: Sized {
    type Success;
    type Error;

    /// Ensure the result satisfies the given predicate.
    fn should<F>(self, predicate: F)
    where
        F: FnOnce(Self) -> bool,
    {
        assert!(predicate(self), "result does not satisfy predicte!");
    }

    /// Ensure the result is ok; return the value.
    fn should_succeed(self) -> Self::Success;

    /// Ensure the result is ok, and the value satisfies the given predicate.
    fn should_succeed_and<F>(self, predicate: F) -> Self::Success
    where
        F: FnOnce(&Self::Success) -> bool,
    {
        let success = self.should_succeed();
        assert!(
            predicate(&success),
            "success as expected, but value does not satisfy predicate!"
        );
        success
    }

    /// Ensure the result is ok, and matches the expect value.
    fn should_succeed_and_equal<U>(self, expect: U) -> Self::Success
    where
        Self::Success: Debug + PartialEq<U>,
        U: Debug,
    {
        let success = self.should_succeed();
        assert_eq!(
            success, expect,
            "success as expected, but with wrong value!"
        );
        success
    }

    /// Ensure the result is ok, but the value doesn't equal the given value.
    fn should_succeed_but_not_equal<U>(self, expect: U) -> Self::Success
    where
        Self::Success: Debug + PartialEq<U>,
        U: Debug,
    {
        let success = self.should_succeed();
        assert_ne!(
            success, expect,
            "success as expected, but with wrong value!"
        );
        success
    }

    /// Ensure the result is error; return the error message;
    fn should_fail(self) -> Self::Error;

    /// Ensure the result is error, and the error satisfies the given predicate.
    fn should_fail_and<F>(self, predicate: F) -> Self::Error
    where
        F: FnOnce(&Self::Error) -> bool,
    {
        let error = self.should_fail();
        assert!(
            predicate(&error),
            "fail as expected, but error does not satisfy predicate!"
        );
        error
    }

    /// Ensure the result is error, and matches the specified error.
    ///
    /// We consider the errors match, if the error message contains the expect
    /// value as a substring.
    fn should_fail_with_error<U>(self, expect: U) -> Self::Error
    where
        Self::Error: ToString,
        U: ToString,
    {
        let error = self.should_fail();
        assert!(
            error.to_string().contains(&expect.to_string()),
            "fail as expected, but with wrong error!"
        );
        error
    }

    /// Ensure the result matches the given result.
    fn should_match<U>(self, expect: GenericResult<U>)
    where
        Self::Success: Debug + PartialEq<U>,
        Self::Error: ToString,
        U: Debug,
    {
        match expect {
            GenericResult::Ok(expect) => {
                self.should_succeed_and_equal(expect);
            },
            GenericResult::Err(expect) => {
                self.should_fail_with_error(expect);
            },
        }
    }
}

impl<T, E> ResultExt for Result<T, E>
where
    T: Debug,
    E: Display,
{
    type Error = E;
    type Success = T;

    fn should_succeed(self) -> Self::Success {
        match self {
            Self::Ok(value) => value,
            Self::Err(err) => panic!("expecting ok, got error: {err}"),
        }
    }

    fn should_fail(self) -> Self::Error {
        match self {
            Self::Err(err) => err,
            Self::Ok(value) => panic!("expecting error, got ok: {value:?}"),
        }
    }
}

impl ResultExt for Outcome {
    type Error = String;
    type Success = Vec<Event>;

    fn should_succeed(self) -> Self::Success {
        match self.result {
            GenericResult::Ok(events) => events,
            GenericResult::Err(error) => panic!("expected success, got error: {error}"),
        }
    }

    fn should_fail(self) -> Self::Error {
        match self.result {
            GenericResult::Err(error) => error,
            GenericResult::Ok(_) => panic!("expected error, got success"),
        }
    }
}

impl ResultExt for TxOutcome {
    type Error = TxError;
    type Success = TxSuccess;

    fn should_succeed(self) -> TxSuccess {
        match self.result {
            GenericResult::Ok(_) => TxSuccess {
                gas_limit: self.gas_limit,
                gas_used: self.gas_used,
                events: self.events,
            },
            GenericResult::Err(err) => panic!("expected success, got error: {err}"),
        }
    }

    fn should_fail(self) -> TxError {
        match self.result {
            GenericResult::Err(error) => TxError {
                gas_limit: self.gas_limit,
                gas_used: self.gas_used,
                error,
                events: self.events,
            },
            GenericResult::Ok(_) => panic!("expected error, got success"),
        }
    }
}
