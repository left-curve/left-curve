use {
    core::error,
    std::{
        backtrace::{self},
        error::Error,
        fmt::{Debug, Display},
        sync::Arc,
    },
};

pub trait Backtraceable {
    fn split(self) -> (String, BT);

    fn backtrace(&self) -> BT;
}

impl Backtraceable for anyhow::Error {
    fn split(self) -> (String, BT) {
        let bt = self.backtrace().into();
        (self.to_string(), bt)
    }

    fn backtrace(&self) -> BT {
        self.backtrace().into()
    }
}

#[derive(Clone)]
pub struct BT(Arc<String>);

impl Default for BT {
    fn default() -> Self {
        (&backtrace::Backtrace::capture()).into()
    }
}

impl From<&backtrace::Backtrace> for BT {
    fn from(bt: &backtrace::Backtrace) -> Self {
        // Copied from anyhow::Error::fmt
        if let backtrace::BacktraceStatus::Captured = bt.status() {
            let mut backtrace = bt.to_string();
            if backtrace.starts_with("stack backtrace:") {
                // Capitalize to match "Caused by:"
                backtrace.replace_range(0..1, "S");
            }
            backtrace.truncate(backtrace.trim_end().len());
            BT(Arc::new(backtrace))
        } else {
            BT(Arc::new("".to_string()))
        }
    }
}

impl Debug for BT {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Display for BT {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone)]
pub struct UnnamedBacktrace<T> {
    pub value: T,
    pub backtrace: BT,
}

impl<T> UnnamedBacktrace<T> {
    pub fn new(t: T) -> Self {
        Self {
            value: t,
            backtrace: BT::default(),
        }
    }

    pub fn new_with_bt(t: T, bt: BT) -> Self {
        Self {
            value: t,
            backtrace: bt,
        }
    }

    pub fn backtrace(&self) -> BT {
        self.backtrace.clone()
    }
}

impl<T> error::Error for UnnamedBacktrace<T> where T: Error {}

impl<T> Display for UnnamedBacktrace<T>
where
    T: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl<T> Debug for UnnamedBacktrace<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.value)
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[grug_macros::backtrace(crate)]
    enum Error {
        #[error(transparent)]
        #[backtrace(fresh)]
        NonBacktraceable(NonBacktraceableError),
        #[error("hi {x}")]
        Named { x: u32 },
        #[error(transparent)]
        Unamed(InnerError),
        #[error("unit")]
        Unit,
    }

    #[grug_macros::backtrace(crate)]
    enum InnerError {
        #[error("my error: {x}")]
        #[backtrace(private_constructor)]
        MyError { x: u32, y: u64 },
    }

    #[derive(Debug, thiserror::Error)]
    enum NonBacktraceableError {
        #[error("my error: {x}")]
        MyError { x: u32 },
    }

    #[test]
    fn test_macro() {
        let inner = NonBacktraceableError::MyError { x: 1 };
        let e: Error = inner.into();

        let a = InnerError::_my_error(1, 2);
    }
}
