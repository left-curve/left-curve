use serde::Serialize;

/// Represents a custom event that contracts may emit.
///
/// It must be serializable to JSON and has a string name.
pub trait EventName: Serialize {
    const NAME: &'static str;
}
