use {
    crate::{
        mailbox::Domain,
        recipients::{RecipientMsg, RecipientQuery, RecipientQueryResponse},
        Addr32,
    },
    anyhow::ensure,
    grug::{
        Addr, Bound, Bounded, Bounds, Bytable, Denom, HexBinary, Inner, NextNumber, NumberConst,
        Part, PrevNumber, Udec128, Uint128, Uint256,
    },
    std::sync::LazyLock,
};

/// The namespace that synthetic tokens will be minted under. The bank contract
/// must give Warp contract admin power over this namespace.
///
/// Synthetic tokens will be given denoms with the format:
///
/// ```plain
/// hyp/{chain_symbol}/{token_symbol}
/// ```
///
/// For examples,
///
/// - `hyp/btc/btc`
/// - `hyp/eth/eth`
/// - `hyp/sol/bonk`
///
/// TODO: The exception to this is alloyed tokens (unimplemented yet).
pub static NAMESPACE: LazyLock<Part> = LazyLock::new(|| Part::new_unchecked("hyp"));

/// The message to be sent via Hyperlane mailbox.
#[derive(Debug)]
pub struct TokenMessage {
    pub recipient: Addr32,
    // Note: In Grug we use `Uint128` to represent token amounts, but the Warp
    // token message uses a 256-bit number to conform to EVM standard. Make sure
    // to account for this when encoding/decoding.
    //
    // Additinally, if someone sends a token from EVM that's more than `Uint128::MAX`,
    // it will error on the destination chain which means the token is stuck on
    // the sender chain.
    pub amount: Uint128,
    pub metadata: HexBinary,
}

impl TokenMessage {
    pub fn encode(&self) -> HexBinary {
        let mut buf = Vec::with_capacity(64 + self.metadata.len());
        buf.extend(self.recipient.inner());
        // Important: cast the amount of 256-bit.
        buf.extend(self.amount.into_next().to_be_bytes());
        buf.extend(self.metadata.inner());
        buf.into()
    }

    pub fn decode(buf: &[u8]) -> anyhow::Result<Self> {
        ensure!(
            buf.len() >= 64,
            "token message should be at least 64 bytes, got: {}",
            buf.len()
        );

        Ok(Self {
            recipient: Addr32::from_inner(buf[0..32].try_into().unwrap()),
            // Important: deserialize the number into 256-bit and try casting
            // into 258-bit. This can fail if the number is too large! Failing
            // here causes collateral tokens being stuck on the origin chain.
            // We should implement frontend check to prevent this.
            amount: Uint256::from_be_bytes(buf[32..64].try_into().unwrap()).checked_into_prev()?,
            metadata: buf[64..].to_vec().into(),
        })
    }
}

#[grug::derive(Serde, Borsh)]
pub struct Route {
    pub address: Addr32,
    pub fee: Uint128,
}

// --------------------------------- messages ----------------------------------

#[grug::derive(Serde)]
pub struct InstantiateMsg {
    /// Address of the mailbox contract.
    pub mailbox: Addr,
}

#[grug::derive(Serde)]
pub enum ExecuteMsg {
    /// Sends tokens to a remote domain.
    ///
    /// Sender must attach exactly one token that is greater than the withdrawal
    /// fee.
    ///
    /// ## Notes:
    ///
    /// We currently don't support:
    ///
    /// - sending more than one tokens at a time;
    /// - specifying a custom hook and hook metadata.
    ///
    /// These should be trivial to implement, but we just don't see a use for
    /// them for now.
    TransferRemote {
        destination_domain: Domain,
        // Note: This means the account the tokens are being sent to, NOT the
        // Hyperlane Warp contract, which is called "route" here and set by the
        // contract owner.
        recipient: Addr32,
        // Note: This is the metadata be to included in the [`TokenMessage`](crate::warp::TokenMessage),
        // NOT the metadata for the hooks.
        metadata: Option<HexBinary>,
    },
    /// Define the recipient contract and withdrawal fee rate for a token on a
    /// destination domain.
    SetRoute {
        denom: Denom,
        destination_domain: Domain,
        route: Route,
        rate_limit: Option<RateLimitConfig>,
    },
    SetRateLimit {
        denom: Denom,
        rate_limit: RateLimitConfig,
    },
    /// Required Hyperlane recipient interface.
    Recipient(RecipientMsg),
}

#[grug::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    /// Query the address of the mailbox contract.
    #[returns(Addr)]
    Mailbox {},
    /// Query the recipient contract for a token on a destination domain.
    #[returns(Route)]
    Route {
        denom: Denom,
        destination_domain: Domain,
    },
    /// Enumerate all routes.
    #[returns(Vec<QueryRoutesResponseItem>)]
    Routes {
        start_after: Option<QueryRoutesPageParam>,
        limit: Option<u32>,
    },
    /// Required Hyperlane recipient interface.
    #[returns(RecipientQueryResponse)]
    Recipient(RecipientQuery),
}

#[grug::derive(Serde)]
pub struct QueryRoutesPageParam {
    pub denom: Denom,
    pub destination_domain: Domain,
}

#[grug::derive(Serde)]
pub struct QueryRoutesResponseItem {
    pub denom: Denom,
    pub destination_domain: Domain,
    pub route: Route,
}

#[grug::derive(Serde)]

pub struct RateLimitConfig {
    pub min_remaining: Uint128,
    pub supply_share: SupplyShare,
}

// -------------------------------- rate-limit ---------------------------------

#[grug::derive(Serde, Borsh)]
pub struct RateLimit {
    pub min_remaining: Uint128,
    pub supply_share: SupplyShare,
    pub remaining: Uint128,
}

#[grug::derive(Serde)]
pub struct SupplyShareBound;

impl Bounds<Udec128> for SupplyShareBound {
    const MAX: Option<Bound<Udec128>> = Some(Bound::Inclusive(Udec128::ONE));
    const MIN: Option<Bound<Udec128>> = Some(Bound::Exclusive(Udec128::ZERO));
}

pub type SupplyShare = Bounded<Udec128, SupplyShareBound>;

// ---------------------------------- events -----------------------------------

#[grug::derive(Serde)]
pub struct TransferRemote {
    pub sender: Addr,
    pub destination_domain: Domain,
    pub recipient: Addr32,
    pub token: Denom,
    pub amount: Uint128,
    pub hook: Option<Addr>,
    pub metadata: Option<HexBinary>,
}

#[grug::derive(Serde)]
pub struct Handle {
    pub recipient: Addr32,
    pub token: Denom,
    pub amount: Uint128,
}
