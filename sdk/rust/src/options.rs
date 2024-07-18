use {crate::SigningKey, grug_types::Addr};

/// Configurations necessary for signing a transaction, including the signing
/// key, sender address, and so on.
pub struct SigningOptions {
    pub signing_key: SigningKey,
    pub sender: Addr,
    pub chain_id: Option<String>,
    pub sequence: Option<u32>,
    // TODO: add options for ADR-070 unordered transactions:
    // https://github.com/left-curve/grug/pull/54
}

/// Configuration on how to choose the admin address when instantiating a
/// contract.
pub enum AdminOption {
    /// Set the admin to a specific address.
    SetToAddr(Addr),
    /// Set the admin to the to-be-deployed contract itself.
    SetToSelf,
    /// Make the admin vacant. In this case, the contract becomes immutable,
    /// i.e. cannot be migrated, other than by the chain's governance.
    SetToNone,
}

impl AdminOption {
    /// Decide the admin address based on ths option chosen.
    pub(crate) fn decide(self, self_addr: &Addr) -> Option<Addr> {
        match self {
            AdminOption::SetToAddr(addr) => Some(addr),
            AdminOption::SetToSelf => Some(self_addr.clone()),
            AdminOption::SetToNone => None,
        }
    }
}
