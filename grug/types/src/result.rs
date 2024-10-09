use {
    crate::{Event, StdError, StdResult},
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
pub trait ResultExt<T, E>: Sized {
    /// Ensure the result satisfies the given predicate.
    fn should<F>(self, predicate: F)
    where
        F: FnOnce(Self) -> bool;

    /// Ensure the result is ok; return the value.
    fn should_succeed(self) -> T;

    /// Ensure the result is ok, and the value satisfies the given predicate.
    fn should_succeed_and<F>(self, predicate: F)
    where
        F: FnOnce(T) -> bool;

    /// Ensure the result is ok, and matches the expect value.
    fn should_succeed_and_equal<U>(self, expect: U)
    where
        T: PartialEq<U>,
        U: Debug;

    /// Ensure the result is ok, but the value doesn't equal the given value.
    fn should_succeed_but_not_equal<U>(self, expect: U)
    where
        T: PartialEq<U>,
        U: Debug;

    /// Ensure the result is error; return the error message;
    fn should_fail(self) -> E;

    /// Ensure the result is error, and the error satisfies the given predicate.
    fn should_fail_and<F>(self, predicate: F)
    where
        F: FnOnce(E) -> bool;

    /// Ensure the result is error, and contains the given substring.
    fn should_fail_with_error<M>(self, msg: M)
    where
        M: ToString;

    /// Ensure the result matches the given result.
    fn should_match<U>(self, expect: GenericResult<U>)
    where
        T: PartialEq<U>,
        U: Debug;
}

macro_rules! impl_result_ext {
    ($t:tt, $e:ty) => {
        fn should<F>(self, predicate: F)
        where
            F: FnOnce(Self) -> bool,
        {
            assert!(predicate(self), "result does not satisfy predicte!");
        }

        fn should_succeed(self) -> T {
            match self {
                $t::Ok(value) => value,
                $t::Err(err) => panic!("expecting ok, got error: {err}"),
            }
        }

        fn should_succeed_and<F>(self, predicate: F)
        where
            F: FnOnce(T) -> bool,
        {
            match self {
                $t::Ok(value) => {
                    assert!(predicate(value), "value does not satisfy predicate!")
                },
                $t::Err(err) => panic!("expecting ok, got error: {err}"),
            }
        }

        fn should_succeed_and_equal<U>(self, expect: U)
        where
            T: PartialEq<U>,
            U: Debug,
        {
            match self {
                $t::Ok(value) => assert_eq!(value, expect, "wrong value!"),
                $t::Err(err) => panic!("expecting ok, got error: {err}"),
            }
        }

        fn should_succeed_but_not_equal<U>(self, expect: U)
        where
            T: PartialEq<U>,
            U: Debug,
        {
            match self {
                $t::Ok(value) => assert_ne!(value, expect, "wrong value!"),
                $t::Err(err) => panic!("expecting ok, got error: {err}"),
            }
        }

        fn should_fail(self) -> $e {
            match self {
                $t::Err(err) => err,
                $t::Ok(value) => panic!("expecting error, got ok: {value:?}"),
            }
        }

        fn should_fail_and<F>(self, predicate: F)
        where
            F: FnOnce($e) -> bool,
        {
            match self {
                $t::Err(err) => {
                    assert!(predicate(err), "error does not satisfy predicate!");
                },
                $t::Ok(value) => panic!("expecting error, got ok: {value:?}"),
            }
        }

        fn should_fail_with_error<M>(self, msg: M)
        where
            M: ToString,
        {
            match self {
                $t::Err(err) => {
                    let expect = msg.to_string();
                    let actual = err.to_string();
                    assert!(
                        actual.contains(&expect),
                        "wrong error! expecting: {expect}, got: {actual}"
                    );
                },
                $t::Ok(value) => panic!("expecting error, got ok: {value:?}"),
            }
        }

        fn should_match<U>(self, expect: GenericResult<U>)
        where
            T: PartialEq<U>,
            U: Debug,
        {
            match (self, expect) {
                ($t::Ok(actual), GenericResult::Ok(expect)) => {
                    assert_eq!(actual, expect, "wrong value!");
                },
                ($t::Err(actual), GenericResult::Err(expect)) => {
                    assert!(
                        actual.to_string().contains(&expect),
                        "wrong error! expecting: {expect}, got {actual}"
                    );
                },
                ($t::Ok(value), GenericResult::Err(_)) => {
                    panic!("expecting error, got ok: {value:?}");
                },
                ($t::Err(err), GenericResult::Ok(_)) => {
                    panic!("expecting ok, got error: {err}");
                },
            }
        }
    };
}

impl<T, E> ResultExt<T, E> for Result<T, E>
where
    T: Debug,
    E: Display,
{
    impl_result_ext!(Result, E);
}

impl<T> ResultExt<T, String> for GenericResult<T>
where
    T: Debug,
{
    impl_result_ext!(GenericResult, String);
}
