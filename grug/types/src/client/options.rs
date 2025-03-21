use crate::Addr;

/// Options on how to set a gas limit on the transaction.
#[derive(Clone, Copy)]
pub enum GasOption {
    /// User has chosen a specific amount of gas wanted.
    Predefined { gas_limit: u64 },
    /// User does not specify a gas limit. The client will simulate the gas
    /// consumption by querying a node, and applying some adjustments.
    Simulate {
        /// Multiply the gas amount by this factor.
        /// This is to account for the inaccuracies in gas simulation in general.
        scale: f64,
        /// After the scaling, increase the simulated gas consumption by this
        /// amount.
        /// This is to account for signature verification cost, which is
        /// typically not included in simulations.
        flat_increase: u64,
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
    pub(crate) fn decide(self, self_addr: Addr) -> Option<Addr> {
        match self {
            AdminOption::SetToAddr(addr) => Some(addr),
            AdminOption::SetToSelf => Some(self_addr),
            AdminOption::SetToNone => None,
        }
    }
}
