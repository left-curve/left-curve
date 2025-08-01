use {
    borsh::{BorshDeserialize, BorshSerialize},
    core::error,
    serde::{Deserialize, Serialize},
    std::{
        backtrace::{self},
        error::Error,
        fmt::{Debug, Display},
        io,
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

#[derive(Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct BT(Arc<String>);

impl BorshSerialize for BT {
    fn serialize<W>(&self, writer: &mut W) -> io::Result<()>
    where
        W: io::Write,
    {
        BorshSerialize::serialize(self.0.as_ref(), writer)
    }
}

impl BorshDeserialize for BT {
    fn deserialize_reader<R>(reader: &mut R) -> io::Result<Self>
    where
        R: io::Read,
    {
        let s = String::deserialize_reader(reader)?;
        Ok(BT(Arc::new(s)))
    }
}

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

#[derive(Serialize, Deserialize, Clone, BorshSerialize, BorshDeserialize, Eq)]
pub struct BacktracedError<T> {
    pub error: T,
    pub backtrace: BT,
}

impl<T> BacktracedError<T> {
    pub fn new(t: T) -> Self {
        Self {
            error: t,
            backtrace: BT::default(),
        }
    }

    pub fn new_with_bt(t: T, bt: BT) -> Self {
        Self {
            error: t,
            backtrace: bt,
        }
    }

    pub fn backtrace(&self) -> BT {
        self.backtrace.clone()
    }
}

impl<T> BacktracedError<T>
where
    T: Display,
{
    pub fn to_string_backtraced(&self) -> String {
        format!("{}\n\n{}", self.error, self.backtrace)
    }
}

impl<T> error::Error for BacktracedError<T> where T: Error {}

impl<T> Display for BacktracedError<T>
where
    T: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.error)
    }
}

impl<T> Debug for BacktracedError<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.error)
    }
}

// manually implement PartialEq for BT to avoid compare the backtrace
impl<T> PartialEq for BacktracedError<T>
where
    T: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.error == other.error
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
