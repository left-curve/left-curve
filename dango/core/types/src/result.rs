use {
    crate::{CheckTxError, CheckTxOutcome, CheckTxSuccess, TxError, TxOutcome, TxSuccess},
    std::fmt::{Debug, Display},
};

/// Addition methods for result types.
/// Useful for testing, improving code readability.
pub trait ResultExt: Sized {
    type Success;
    type Error;

    /// Ensure the result satisfies the given predicate.
    fn should<F>(self, predicate: F)
    where
        Self: Debug,
        F: FnOnce(&Self) -> bool,
    {
        assert!(
            predicate(&self),
            "result does not satisfy predicte! result: {self:?}"
        );
    }

    /// Ensure the result is ok; return the value.
    fn should_succeed(self) -> Self::Success;

    /// Ensure the result is ok, and the value satisfies the given predicate.
    fn should_succeed_and<F>(self, predicate: F) -> Self::Success
    where
        Self::Success: Debug,
        F: FnOnce(&Self::Success) -> bool,
    {
        let success = self.should_succeed();
        assert!(
            predicate(&success),
            "success as expected, but value does not satisfy predicate! value: {success:?}"
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
            "success as expected, but with different value! expecting: {expect:?}, got: {success:?}"
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
            "success as expected, but with same value! expecting: {expect:?}, got: {success:?}"
        );
        success
    }

    /// Ensure the result is error; return the error message;
    fn should_fail(self) -> Self::Error;

    /// Ensure the result is error, and the error satisfies the given predicate.
    fn should_fail_and<F>(self, predicate: F) -> Self::Error
    where
        Self::Error: Display,
        F: FnOnce(&Self::Error) -> bool,
    {
        let error = self.should_fail();
        assert!(
            predicate(&error),
            "fail as expected, but error does not satisfy predicate! error: {error}"
        );
        error
    }

    /// Ensure the result is error, and matches the specified error.
    ///
    /// We consider the errors match, if the error message contains the expect
    /// value as a substring.
    fn should_fail_with_error<U>(self, expect: U) -> Self::Error
    where
        Self::Error: Display,
        U: Display,
    {
        let error = self.should_fail();
        assert!(
            error.to_string().contains(&expect.to_string()),
            "fail as expected, but with wrong error! expecting: {expect}, got: {error}"
        );
        error
    }

    /// Ensure the result matches the given result.
    fn should_match<T, E>(self, expect: Result<T, E>)
    where
        Self::Success: Debug + PartialEq<T>,
        Self::Error: Display,
        T: Debug,
        E: Display,
    {
        match expect {
            Ok(expect) => {
                self.should_succeed_and_equal(expect);
            },
            Err(expect) => {
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

impl ResultExt for TxOutcome {
    type Error = TxError;
    type Success = TxSuccess;

    fn should_succeed(self) -> TxSuccess {
        match self.result {
            Ok(_) => TxSuccess {
                gas_limit: self.gas_limit,
                gas_used: self.gas_used,
                events: self.events,
            },
            Err(err) => panic!("expected success, got error: {err}"),
        }
    }

    fn should_fail(self) -> TxError {
        match self.result {
            Err(error) => TxError {
                gas_limit: self.gas_limit,
                gas_used: self.gas_used,
                error,
                events: self.events,
            },
            Ok(_) => panic!("expected error, got success"),
        }
    }
}

impl ResultExt for CheckTxOutcome {
    type Error = CheckTxError;
    type Success = CheckTxSuccess;

    fn should_succeed(self) -> Self::Success {
        match self.result {
            Ok(_) => CheckTxSuccess {
                gas_limit: self.gas_limit,
                gas_used: self.gas_used,
                events: self.events,
            },
            Err(err) => panic!("expecting success, got error: {err}"),
        }
    }

    fn should_fail(self) -> Self::Error {
        match self.result {
            Err(error) => CheckTxError {
                gas_limit: self.gas_limit,
                gas_used: self.gas_used,
                error: error.to_string(),
                events: self.events,
            },
            Ok(_) => panic!("expecting error, got success"),
        }
    }
}
