use {crate::gateway::NAMESPACE, grug::Denom, std::slice};

pub trait BridgeDenom {
    fn is_remote(&self) -> bool;
}

impl BridgeDenom for Denom {
    /// Returns true if the denom is a remote denom (i.e. starts with the `bridge` namespace).
    fn is_remote(&self) -> bool {
        self.starts_with(slice::from_ref(&NAMESPACE))
    }
}
