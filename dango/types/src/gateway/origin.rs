use {
    crate::gateway::NAMESPACE,
    grug::{Denom, Part},
    std::slice,
};

#[grug::derive(Serde)]
#[derive(PartialOrd, Ord)]
pub enum Origin {
    /// Token is issued natively on Dango.
    Local(Denom),
    /// The token was received through a remote chain.
    Remote(Part),
}

pub trait Traceable {
    fn is_remote(&self) -> bool;
}

impl Traceable for Denom {
    /// Returns `true` if the denom is a remote denom (i.e. starts with the `bridge` namespace).
    fn is_remote(&self) -> bool {
        self.starts_with(slice::from_ref(&NAMESPACE))
    }
}
