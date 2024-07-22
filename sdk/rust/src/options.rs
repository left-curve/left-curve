use {crate::SigningKey, grug_types::Addr};

/// Configurations necessary for signing a transaction, including the signing
/// key, sender address, and so on.
pub struct SigningOption<'a> {
    pub signing_key: &'a SigningKey,
    pub sender: Addr,
    pub chain_id: Option<String>,
    pub sequence: Option<u32>,
    // TODO: add options for ADR-070 unordered transactions:
    // https://github.com/left-curve/grug/pull/54
}

/// Options on how to set a gas limit on the transaction.
#[derive(Clone, Copy)]
pub enum GasOption {
    /// User has chosen a specific amount of gas wanted.
    Predefined { gas_limit: u64 },
    /// User does not specify a gas limit. The client will simulate the gas
    /// consumption by querying a node, and applying some adjustments.
    Simulate {
        /// Increase the simulated gas consumption by a flat amount. This is to
        /// account for signature verification cost, which is typically not
        /// included in simulations.
        flat_increase: u64,
        /// After the flat increase, multiply the gas amount by a factor.
        /// This is to account for the inaccuracies in gas simulation in general.
        scale: f64,
    },
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
