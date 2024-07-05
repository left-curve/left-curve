use {
    anyhow::{bail, ensure},
    grug_app::{AppError, AppResult},
    std::fmt::Debug,
};

/// A wrapper over the `AppResult`, providing two convenience methods to make
/// our tests more readable.
pub struct TestResult<T> {
    inner: AppResult<T>,
}

impl<T> From<AppResult<T>> for TestResult<T> {
    fn from(inner: AppResult<T>) -> Self {
        Self { inner }
    }
}

impl<T> TestResult<T>
where
    T: Debug,
{
    /// Ensure the result is ok; return the value.
    pub fn should_succeed(self) -> anyhow::Result<T> {
        match self.inner {
            Ok(value) => Ok(value),
            Err(err) => bail!("expecting ok, got error: {err}"),
        }
    }

    /// Ensure the result is ok, and matches the expect value.
    pub fn should_succeed_and_equal<V>(self, expect: V) -> anyhow::Result<()>
    where
        T: PartialEq<V>,
        V: Debug,
    {
        match self.inner {
            Ok(value) => {
                ensure!(
                    value == expect,
                    "wrong value! expecting: {expect:?}, got: {value:?}"
                );
            },
            Err(err) => bail!("expecting ok, got error: {err}"),
        }
        Ok(())
    }

    /// Ensure the result is error; return the error;
    pub fn should_fail(self) -> anyhow::Result<AppError> {
        match self.inner {
            Err(err) => Ok(err),
            Ok(value) => bail!("expecting error, got ok: {value:?}"),
        }
    }

    /// Ensure the result is error, and contains the given message.
    pub fn should_fail_with_error(self, msg: impl ToString) -> anyhow::Result<()> {
        match self.inner {
            Err(err) => {
                // Here we stringify the error and check for the existence of
                // the substring, instead of utilizing the Rust type system.
                //
                // Have to go with this approach because errors emitted by the
                // contract are converted to strings (as `GenericResult`) when
                // passed through the FFI, at which time they lost their types.
                let expect = msg.to_string();
                let actual = err.to_string();
                ensure!(
                    actual.contains(&expect),
                    "wrong error! expecting: {expect}, got: {actual}"
                );
            },
            Ok(value) => bail!("expecting error, got ok: {value:?}"),
        }
        Ok(())
    }
}
