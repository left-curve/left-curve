use {
    dango_types::{
        account_factory::Username,
        auth::Key,
        bank,
        bitcoin::{BitcoinAddress, Network},
        config::Hyperlane,
        dex::PairUpdate,
        gateway::{RateLimit, Remote, WithdrawalFee},
        lending::InterestRateModel,
        oracle::PriceSource,
        taxman,
    },
    grug::{Addr, Coin, Coins, Denom, Duration, Hash256, NonEmpty, Order, Part, Uint128},
    hyperlane_types::{isms::multisig::ValidatorSet, mailbox::Domain},
    pyth_types::{GuardianSet, GuardianSetIndex},
    std::collections::{BTreeMap, BTreeSet},
};

pub type GenesisUsers = BTreeMap<Username, GenesisUser>;

pub type Addresses = BTreeMap<Username, Addr>;

#[grug::derive(Serde)]
pub struct Contracts {
    pub account_factory: Addr,
    pub bank: Addr,
    pub dex: Addr,
    pub gateway: Addr,
    pub hyperlane: Hyperlane<Addr>,
    pub lending: Addr,
    pub oracle: Addr,
    pub taxman: Addr,
    pub vesting: Addr,
    pub warp: Addr,
    pub bitcoin: Addr,
}

#[derive(Clone, Copy)]
pub struct Codes<T> {
    pub account_factory: T,
    pub account_margin: T,
    pub account_multi: T,
    pub account_spot: T,
    pub bank: T,
    pub dex: T,
    pub gateway: T,
    pub hyperlane: Hyperlane<T>,
    pub lending: T,
    pub oracle: T,
    pub taxman: T,
    pub vesting: T,
    pub warp: T,
    pub bitcoin: T,
}
pub struct GenesisUser {
    pub key: Key,
    pub key_hash: Hash256,
    pub dango_balance: Uint128,
}

pub struct GenesisOption {
    pub grug: GrugOption,
    pub account: AccountOption,
    pub bank: BankOption,
    pub dex: DexOption,
    pub gateway: GatewayOption,
    pub hyperlane: HyperlaneOption,
    pub lending: LendingOption,
    pub oracle: OracleOption,
    pub vesting: VestingOption,
    pub bitcoin: BitcoinOption,
}

pub struct GrugOption {
    /// A username whose genesis spot account is to be appointed as the owner.
    /// We expect to transfer ownership to a multisig account afterwards.
    pub owner_username: Username,
    /// Gas fee configuration.
    pub fee_cfg: taxman::Config,
    /// The maximum age a contract bytecode can remain orphaned (not used by any
    /// contract).
    /// Once this time is elapsed, the code is deleted and must be uploaded again.
    pub max_orphan_age: Duration,
}

pub struct AccountOption {
    /// Initial users and their balances.
    /// For each genesis user will be created a spot account.
    pub genesis_users: BTreeMap<Username, GenesisUser>,
    /// The minimum deposit required to onboard a user.
    pub minimum_deposit: Coins,
}

pub struct BankOption {
    /// Metadata of tokens.
    pub metadatas: BTreeMap<Denom, bank::Metadata>,
}

pub struct DexOption {
    /// Initial Dango DEX trading pairs.
    pub pairs: Vec<PairUpdate>,
}

pub struct GatewayOption {
    pub routes: BTreeSet<(Part, Remote)>,
    pub rate_limits: BTreeMap<Denom, RateLimit>,
    pub rate_limit_refresh_period: Duration,
    pub withdrawal_fees: Vec<WithdrawalFee>,
}

pub struct HyperlaneOption {
    /// Hyperlane domain ID of the local domain.
    pub local_domain: Domain,
    /// Hyperlane validator sets for remote domains.
    pub ism_validator_sets: BTreeMap<Domain, ValidatorSet>,
    /// Hyperlane validator announce fee rate.
    pub va_announce_fee_per_byte: Coin,
}

pub struct LendingOption {
    /// Initial Dango lending markets.
    pub markets: BTreeMap<Denom, InterestRateModel>,
}

pub struct OracleOption {
    /// Oracle price sources.
    pub pyth_price_sources: BTreeMap<Denom, PriceSource>,
    /// Wormhole guardian sets that will sign Pyth VAA messages.
    pub wormhole_guardian_sets: BTreeMap<GuardianSetIndex, GuardianSet>,
}

pub struct VestingOption {
    /// Cliff for Dango token unlocking.
    pub unlocking_cliff: Duration,
    /// Period for Dango token unlocking.
    pub unlocking_period: Duration,
}

pub struct BitcoinOption {
    pub network: Network,
    pub vault: BitcoinAddress,
    pub guardians: NonEmpty<BTreeSet<Username>>,
    pub threshold: u8,
    pub sats_per_vbyte: Uint128,
    pub outbound_fee: Uint128,
    pub outbound_strategy: Order,
}
