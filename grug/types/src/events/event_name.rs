/// Represents a type that has a name associated with it. Necessary for the type
/// to be emitted as a [`ContractEvent`](crate::ContractEvent).
pub trait EventName {
    const NAME: &'static str;
}
