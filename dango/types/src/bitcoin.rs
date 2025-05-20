use {
    corepc_client::bitcoin::Network,
    grug::{Addr, Denom, Hash256, HexBinary, NonEmpty, Order, Uint128},
    std::{
        collections::{BTreeMap, BTreeSet},
        sync::LazyLock,
    },
};

pub const OVERHEAD_SIZE: Uint128 = Uint128::new(11);
pub const INPUT_SIZE: Uint128 = Uint128::new(105);
pub const OUTPUT_SIZE: Uint128 = Uint128::new(43);

pub const NAMESPACE: &str = "bitcoin";

pub const SUBDENOM: &str = "satoshi";

pub const DENOM: LazyLock<Denom> = LazyLock::new(|| Denom::new_unchecked([NAMESPACE, SUBDENOM]));

/// Bitcoin address of the P2WPKH (pay to witness public key hash) type, which
/// is 20-bytes long.
// TODO: There are other types of Bitcoin addresses.
pub type BitcoinAddress = String;

/// An Bitcoin signature.
pub type BitcoinSignature = HexBinary;

/// The index of the output in a Bitcoin transaction.
pub type Vout = u32;

#[grug::derive(Serde)]
pub struct Config {
    pub network: Network,
    pub vault: BitcoinAddress,
    pub guardians: NonEmpty<BTreeSet<Addr>>,
    pub threshold: u8,
    /// The amount of Sats for each vByte to calculate the fee.
    pub sats_per_vbyte: Uint128,
    /// For outbound transactions, a flat fee deducted from the withdraw amount.
    ///
    /// We expect this to be updated often to reflect the gas price on Bitcoin
    /// network, and roughly inline with the withdrawal fee on major centralized
    /// exchanges. For example:
    ///
    /// - [Binance](https://www.binance.com/en/fee/cryptoFee)
    pub outbound_fee: Uint128,
    /// Strategy for choosing the UTXOs as inputs for outbound transactions.
    ///
    /// During periods of high gas price on Bitcoin network, we want to minimize
    /// the number of input UTXOs to save on gas fees. To achieve this, use `Order::Descending`
    /// so that we use the biggest UTXOs first.
    ///
    /// During period of low gas price, we may want to take the opportunity to
    /// consolidate the many small UTXOs into a few big ones. To achieve this,
    /// use `Order::Ascending`.
    pub outbound_strategy: Order,
    // TODO: minimum deposit?
}

#[grug::derive(Serde, Borsh)]
pub struct Transaction {
    pub inputs: BTreeMap<(Hash256, Vout), Uint128>,
    pub outputs: BTreeMap<BitcoinAddress, Uint128>,
    pub fee: Uint128,
}

#[grug::derive(Serde)]
pub struct Utxo {
    pub transaction_hash: Hash256,
    pub vout: Vout,
    pub amount: Uint128,
}

#[grug::derive(Serde)]
pub struct InstantiateMsg {
    pub config: Config,
}

#[grug::derive(Serde)]
pub enum ExecuteMsg {
    /// Update the guardian addresses and/or threshold.
    ///
    /// Can only be called by the chain owner.
    UpdateConfig {
        sats_per_vbyte: Option<Uint128>,
        outbound_fee: Option<Uint128>,
        outbound_strategy: Option<Order>,
        // TODO: Allow changing the vault address and guardian set? This requires resetting the UTXO set.
    },
    /// Observe an inbound transaction.
    ///
    /// Can only be called by the guardians.
    ObserveInbound {
        /// The Bitcoin transaction hash.
        transaction_hash: Hash256,
        /// The transaction's output index.
        vout: Vout,
        /// The transaction's UTXO amount.
        amount: Uint128,
        /// The recipient of the inbound transfer.
        ///
        /// In case of a user making a deposit, he must indicate the recipient
        /// address in the transaction's memo. The guardian must report this
        /// recipient.
        ///
        /// Other kinds of inbound transactions do not have a recipient. For
        /// example, an outbound transaction may have an excess amount. Or, the
        /// operator may top up the multisig's balance to cover gas cost.
        recipient: Option<Addr>,
    },
    /// Withdraw Bitcoin buy burning the synthetic token on Dango.
    ///
    /// Can be called by anyone. Caller must send a non-zero amount of synthetic
    /// Bitcoin token and nothing else.
    ///
    /// Outbound transactions are pushed into a queue. Every once in a while (as
    /// defined by the contract's cronjob frequency), the contract finds all
    /// withdrawals in the queue, and builds a transaction.
    Withdraw {
        /// The recipient Bitcoin address.
        ///
        /// TODO: There are various bitcoin address formats. Should we enforce one?
        ///
        /// https://bitcoin.stackexchange.com/questions/119736
        recipient: BitcoinAddress,
    },
    /// Authorize an outbound transaction.
    ///
    /// Can only be called by the guardians.
    AuthorizeOutbound {
        /// Identifier of the outbound transaction.
        ///
        /// Each outbound transaction is identified by a incremental ID.
        /// This ID is generated when a user calls the `withdraw` method.
        id: u32,
        /// A Bitcoin signature authorizing the outbound transaction.
        ///
        /// Once a threshold number of signatures has been received, a worker
        /// will pick it up and broadcast the transaction on the Bitcoin network.
        signatures: Vec<BitcoinSignature>,
    },
}

#[grug::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    /// Query the contract configurations.
    #[returns(Config)]
    Config {},
    /// Enumerate the UTXOs spendable by the multisig, sorted by amount.
    #[returns(Vec<Utxo>)]
    Utxos {
        start_after: Option<Utxo>,
        limit: Option<u32>,
        order: Order,
    },
    /// Enumerate pending outbound transactions in the queue.
    #[returns(BTreeMap<BitcoinAddress, Uint128>)]
    OutboundQueue {
        start_after: Option<BitcoinAddress>,
        limit: Option<u32>,
    },
    /// Query the last outbound transaction ID.
    #[returns(u32)]
    LastOutboundTransactionId {},
    /// Query an outbound transaction by ID.
    #[returns(Transaction)]
    OutboundTransaction { id: u32 },
    /// Enumerate all outbound transactions.
    #[returns(BTreeMap<u32, Transaction>)]
    OutboundTransactions {
        start_after: Option<u32>,
        limit: Option<u32>,
    },
    /// Query the signatures for a single outbound transactions by ID.
    #[returns(BTreeMap<Addr, BitcoinSignature>)]
    OutboundSignature { id: u32 },
    /// Enumerate all signatures for all outbound transactions.
    #[returns(BTreeMap<u32, BTreeMap<Addr, BitcoinSignature>>)]
    OutboundSignatures {
        start_after: Option<u32>,
        limit: Option<u32>,
    },
}

/// Event indicating an inbound transaction has been observed by a threshold
/// number of guardians.
#[grug::derive(Serde)]
#[grug::event("inbound_confirmed")]
pub struct InboundConfirmed {
    pub transaction_hash: Hash256,
    pub amount: Uint128,
    pub recipient: Option<Addr>,
}

/// Event indicating an outbound transaction has been requested, pending signatures
/// from the guardians.
///
/// Guardian worker should observe this event and sign accordingly.
#[grug::derive(Serde)]
#[grug::event("outbound_requested")]
pub struct OutboundRequested {
    pub id: u32,
    pub transaction: Transaction,
}

/// Event indicating an outbound transaction has received a threshold number of
/// signatures, and is ready to be broadcasted on the Bitcoin network.
///
/// Broadcaster worker should observe this event and broadcast accordingly.
#[grug::derive(Serde)]
#[grug::event("outbound_confirmed")]
pub struct OutboundConfirmed {
    pub id: u32,
    pub transaction: Transaction,
    pub signatures: BTreeMap<Addr, Vec<BitcoinSignature>>,
}
