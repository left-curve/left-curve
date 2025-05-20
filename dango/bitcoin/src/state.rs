use {
    dango_types::bitcoin::{BitcoinSignature, Config, Transaction, Vout},
    grug::{
        Addr, Counter, Empty, Hash256, IndexedMap, Item, Map, Serde, Set, Uint128, UniqueIndex,
    },
    std::collections::{BTreeMap, BTreeSet},
};

pub const CONFIG: Item<Config, Serde> = Item::new("config");

/// Inbound transactions that have not received threshold number of votes.
///
/// ```plain
/// (transaction_hash, amount, recipient) => voted_guardians
/// ```
pub const INBOUNDS: Map<(Hash256, Vout, Uint128, Option<Addr>), BTreeSet<Addr>> =
    Map::new("inbound");

/// UTXOs owned by the multisig, available to be spent for outbound transactions.
///
/// ```plain
/// amount => transaction_hash
/// ```
///
/// TODO: We should create `IndexedSet` for this.
pub const UTXOS: IndexedMap<(Uint128, Hash256, Vout), Empty, UtxoIndexes> =
    IndexedMap::new("utxo", UtxoIndexes {
        transaction_hash: UniqueIndex::new(|(_, hash, _), _| *hash, "utxo", "utxo__hash"),
    });

/// UTXOs that have been processed by the multisig.
/// This is used to prevent double spending.
pub const PROCESSED_UTXOS: Set<(Hash256, Vout)> = Set::new("processed_utxo");

/// Outbound transactions that have not received threshold number of signatures.
///
/// ```plain
/// recipient_bitcoin_address => amount
/// ```
pub const OUTBOUND_QUEUE: Map<&[u8], Uint128> = Map::new("outbound");

pub const NEXT_OUTBOUND_ID: Counter<u32> = Counter::new("next_outbound_id", 0, 1);

pub const OUTBOUNDS: Map<u32, Transaction> = Map::new("outbound");

pub const SIGNATURES: Map<u32, BTreeMap<Addr, Vec<BitcoinSignature>>> = Map::new("signature");

#[grug::index_list((Uint128, Hash256, Vout), Empty)]
pub struct UtxoIndexes<'a> {
    pub transaction_hash: UniqueIndex<'a, (Uint128, Hash256, Vout), Hash256, Empty>,
}
