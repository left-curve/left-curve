use grug::{Addr, Denom};

/// The namespace that lending pool uses.
pub const NAMESPACE: &str = "lending";

#[grug::derive(Serde)]
pub struct InstantiateMsg {
    pub whitelisted_denoms: Vec<Denom>,
}

#[grug::derive(Serde)]
pub enum ExecuteMsg {
    /// Whitelist a denom. Can only be invoked by the owner.
    WhitelistDenom(Denom),

    /// Delist a denom. Can only be invoked by the owner.
    DelistDenom(Denom),

    /// Deposit tokens into the lending pool.
    Deposit {
        /// The optional recipient of the minted LP tokens. If not set, the
        /// sender's address will be used.
        recipient: Option<Addr>,
    },

    /// Withdraw tokens from the lending pool by redeeming LP tokens. LP tokens
    /// should be sent to the contract together with this message.
    Withdraw {
        /// The optional recipient of the withdrawn tokens. If not set, the
        /// sender's address will be used.
        recipient: Option<Addr>,
    },
}

#[grug::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    /// Get the list of whitelisted denoms.
    #[returns(Vec<Denom>)]
    WhitelistedDenoms {
        /// The maximum number of denoms to return, if not set, will attempt to
        /// return all denoms.
        limit: Option<u32>,

        /// The denom to start paginating after, if not set, will start from the
        /// first denom.
        start_after: Option<Denom>,
    },
}
