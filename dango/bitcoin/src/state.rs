use {
    dango_types::bitcoin::{BitcoinAddress, BitcoinSignature, Config, Transaction, Vout},
    grug::{Addr, Counter, Hash256, HexByteArray, Item, Map, Serde, Set, Uint128},
    std::collections::{BTreeMap, BTreeSet},
};

pub const CONFIG: Item<Config, Serde> = Item::new("config");

/// Inbound transactions that have not received threshold number of votes.
///
/// ```plain
/// (transaction_hash, amount, recipient) => voted_guardians
/// ```
pub const INBOUNDS: Map<(Hash256, Vout, Uint128, Option<Addr>), BTreeSet<HexByteArray<33>>> =
    Map::new("inbound");

/// UTXOs owned by the multisig, available to be spent for outbound transactions.
///
/// ```plain
/// (amount, transaction_hash, vout)
/// ```
pub const UTXOS: Set<(Uint128, Hash256, Vout)> = Set::new("utxo");

/// UTXOs that have been processed by the multisig and accredited to the user.
/// This is used to prevent double spending.
pub const PROCESSED_UTXOS: Set<(Hash256, Vout)> = Set::new("processed_utxo");

/// Outbound transactions that have not received threshold number of signatures.
///
/// ```plain
/// recipient_bitcoin_address => amount
/// ```
pub const OUTBOUND_QUEUE: Map<BitcoinAddress, Uint128> = Map::new("outbound_queue");

pub const OUTBOUND_ID: Counter<u32> = Counter::new("outbound_id", 0, 1);

/// Outbound transactions that have been processed and need to be signed from validators
/// before broadcast to Bitcoin.
pub const OUTBOUNDS: Map<u32, Transaction> = Map::new("outbound");

/// Signatures for outbound transactions that have been signed by validators.
pub const SIGNATURES: Map<u32, BTreeMap<HexByteArray<33>, Vec<BitcoinSignature>>> =
    Map::new("signature");
