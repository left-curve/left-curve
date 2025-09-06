use {
    crate::gateway::NAMESPACE,
    grug::{Denom, Part},
};

#[grug::derive(Serde)]
#[derive(PartialOrd, Ord)]
pub enum Origin {
    /// The token was received through a remote chain.
    Remote(Part),
    /// Token is native on Dango.
    Native(Denom),
}

pub trait Traceable {
    fn is_remote(&self) -> bool;
}

impl Traceable for Denom {
    /// Returns `true` if the denom is a remote denom (i.e. starts with the `bridge` namespace).
    fn is_remote(&self) -> bool {
        self.starts_with(&[NAMESPACE.clone()])
    }
}
