use {
    dango_types::bitcoin::{
        BitcoinAddress, BitcoinSignature, Config, Recipient, Transaction, Vout,
    },
    grug::{
        Addr, Counter, Hash256, HexByteArray, IndexedMap, Item, Map, Serde, Set, Uint128,
        UniqueIndex,
    },
    std::collections::{BTreeMap, BTreeSet},
};

pub const CONFIG: Item<Config, Serde> = Item::new("config");

/// Inbound transactions that have not received threshold number of votes.
///
/// ```plain
/// (transaction_hash, vout, amount, Recipient) => voted_guardians
/// ```
pub const INBOUNDS: Map<(Hash256, Vout, Uint128, Recipient), BTreeSet<HexByteArray<33>>> =
    Map::new("inbound");

pub const ADDRESSES: IndexedMap<Addr, u64, AddressIndexes> =
    IndexedMap::new("address", AddressIndexes {
        address_index: UniqueIndex::new(|_, id| *id, "address", "address__idx"),
    });

/// For each dango address, we assign a unique index to generate different Bitcoin addresses.
pub const ADDRESS_INDEX: Counter<u64> = Counter::new("address_index", 1, 1);

/// UTXOs owned by the multisig, available to be spent for outbound transactions.
///
/// ```plain
/// (amount, transaction_hash, vout) => Recipient
/// ```
pub const UTXOS: Map<(Uint128, Hash256, Vout), Recipient> = Map::new("utxo");

/// UTXOs that have been processed by the multisig and accredited to the user.
/// This is used to prevent double spending.
pub const PROCESSED_UTXOS: Set<(Hash256, Vout)> = Set::new("processed_utxo");

/// Outbound requests: they will be processed during the CronExecute.
///
/// ```plain
/// recipient_bitcoin_address => amount
/// ```
pub const OUTBOUND_QUEUE: Map<BitcoinAddress, Uint128> = Map::new("outbound_queue");

pub const OUTBOUND_ID: Counter<u32> = Counter::new("outbound_id", 0, 1);

/// Outbound transactions that have been processed and need to be signed from guardians
/// before broadcast to Bitcoin.
pub const OUTBOUNDS: Map<u32, Transaction> = Map::new("outbound");

/// Signatures for outbound transactions that have been signed by guardians.
pub const SIGNATURES: Map<u32, BTreeMap<HexByteArray<33>, Vec<BitcoinSignature>>> =
    Map::new("signature");

#[grug::index_list(Addr, u64)]
pub struct AddressIndexes<'a> {
    pub address_index: UniqueIndex<'a, Addr, u64, u64>,
}
