use {
    borsh::{BorshDeserialize, BorshSerialize},
    core::error,
    serde::{Deserialize, Serialize, de::Visitor},
    std::{
        backtrace::{self},
        error::Error,
        fmt::{Debug, Display},
        io,
        sync::Arc,
    },
};

pub trait Backtraceable {
    fn into_generic_backtraced_error(self) -> BacktracedError<String>;
    fn backtrace(&self) -> BT;
    fn error(&self) -> String;
}

impl Backtraceable for anyhow::Error {
    fn into_generic_backtraced_error(self) -> BacktracedError<String> {
        let bt = self.backtrace().into();
        BacktracedError::new_with_bt(self.to_string(), bt)
    }

    fn backtrace(&self) -> BT {
        self.backtrace().into()
    }

    fn error(&self) -> String {
        self.to_string()
    }
}

// ------------------------------------ BT -------------------------------------

#[derive(Eq, PartialEq, Clone)]
pub struct BT(Arc<String>);

impl BT {
    pub fn capture_if_empty(self) -> Self {
        if self.0.as_ref().is_empty() {
            Self::default()
        } else {
            self
        }
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
        Ok(BT(Arc::new(String::deserialize_reader(reader)?)))
    }
}

struct BTVisitor;

impl<'de> Visitor<'de> for BTVisitor {
    type Value = BT;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a string-encoded backtrace")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(BT(Arc::new(v.to_string())))
    }
}

impl Serialize for BT {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.0.as_ref())
    }
}

impl<'de> Deserialize<'de> for BT {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(BTVisitor)
    }
}

// ----------------------------- Backtraced Error ------------------------------

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

    pub fn new_without_bt(t: T) -> Self {
        Self {
            error: t,
            backtrace: BT(Arc::new("".to_string())),
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
    T: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}\n\n{}", self.error, self.backtrace)
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
        #[backtrace(new)]
        NonBacktraceable(NonBacktraceableError),
        #[error("hi {x}")]
        Named { x: u32 },
        #[error(transparent)]
        Unnamed(InnerError),
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

        let _: Error = inner.into();
        InnerError::_my_error(1, 2);
    }
}
