use {
    grug::{Addr, Coins, Denom, LengthBounded, Part, Uint128},
    std::collections::BTreeMap,
};

#[grug::derive(Serde)]
pub struct InstantiateMsg {
    /// Initial account balances.
    pub balances: BTreeMap<Addr, Coins>,
    /// Initial namespace ownerships.
    pub namespaces: BTreeMap<Part, Addr>,
    /// Initial denom metadatas.
    pub metadatas: BTreeMap<Denom, Metadata>,
}

#[grug::derive(Serde)]
pub enum ExecuteMsg {
    /// Grant the control of a namespace to an account.
    /// Can only be called by the chain owner.
    ///
    /// Currently, we don't support:
    ///
    /// 1. granting the top-level namespace;
    /// 2. chain owner canceling a grant;
    /// 3. namespace owner renouncing or transferring a grant;
    /// 4. a namespace to have more than one owner;
    /// 5. before-send hooks.
    ///
    /// We may implement some of these in the future.
    GrantNamespace { namespace: Part, owner: Addr },
    /// Set metadata of a denom.
    /// Can only be called by the namespace owner, or the chain owner in case of
    /// top-level denoms.
    SetMetadata { denom: Denom, metadata: Metadata },
    /// Mint tokens of the specified amount to a recipient.
    /// Can only be called by the namespace owner.
    Mint {
        to: Addr,
        denom: Denom,
        amount: Uint128,
    },
    /// Burn tokens of the specified amount from an account.
    /// Can only be called by the namespace owner.
    Burn {
        from: Addr,
        denom: Denom,
        amount: Uint128,
    },
    /// Forcily transfer a coin from an account to a receiver.
    /// Can only be called by the chain's taxman contract.
    /// Used by taxman to withhold pending transaction fees.
    ///
    /// Note: The `receive` method isn't invoked when calling this.
    ForceTransfer {
        from: Addr,
        to: Addr,
        denom: Denom,
        amount: Uint128,
    },
    /// Transfer coins to multiple recipients at once.
    BatchTransfer(BTreeMap<Addr, Coins>),
}

#[grug::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    /// Query the owner of a namespace.
    #[returns(Addr)]
    Namespace { namespace: Part },
    /// Enumerate owners of all namespaces.
    #[returns(BTreeMap<Part, Addr>)]
    Namespaces {
        start_after: Option<Part>,
        limit: Option<u32>,
    },
    /// Query the metadata of a denom.
    #[returns(Metadata)]
    Metadata { denom: Denom },
    /// Enumerate metadata of all denoms.
    #[returns(BTreeMap<Denom, Metadata>)]
    Metadatas {
        start_after: Option<Denom>,
        limit: Option<u32>,
    },
}

#[grug::derive(Serde, Borsh)]
pub struct Metadata {
    // The length limits were arbitrarily chosen and can be adjusted.
    pub name: LengthBounded<String, 1, 32>,
    pub symbol: LengthBounded<String, 1, 16>,
    pub description: Option<LengthBounded<String, 1, 140>>,
    pub decimals: u8,
}
