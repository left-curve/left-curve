use {
    super::{BlockStore, ranges::StoredRanges},
    anyhow::anyhow,
    async_trait::async_trait,
    dango_indexer_historical_types::{AnyResult, BlockData},
    rocksdb::{
        BlockBasedOptions, Cache, ColumnFamily, DB, DBCompressionType, IteratorMode, Options,
        WriteBatch,
    },
    std::{path::Path, sync::Mutex},
};

/// The default CF holds metadata only (the topology checkpoint); named here
/// because `open_cf_with_opts` must list every column family explicitly.
const CF_DEFAULT: &str = "default";

/// Column family for the raw blocks: key = `height` big-endian (so RocksDB's
/// lexicographic order *is* height order), value = borsh-encoded [`BlockData`].
const CF_BLOCKS: &str = "blocks";

/// Key in the default CF holding the topology checkpoint — the [`StoredRanges`]
/// serialized as a borsh `Vec<(u64, u64)>`.
const TOPOLOGY_KEY: &[u8] = b"topology";

/// Persistent [`BlockStore`] over a local RocksDB.
///
/// Blocks live in the `blocks` CF keyed by big-endian height; the stored-height
/// topology is mirrored in RAM (for O(1) frontier/gap queries) and checkpointed
/// to the default CF in the **same atomic batch** as each block, so boot reads
/// the frontier in O(#ranges) rather than scanning every height. The raw blocks
/// are re-fetchable, so a missing checkpoint is recovered by rebuilding the
/// topology from the block keys.
///
/// Single-process by design: the detached `RemoteBlockSource` owns it, the
/// coordinator is the only writer, and the query service reaches blocks through
/// the same in-process source — so an embedded store is reachable, not a limit.
pub struct RocksdbBlockStore {
    db: DB,
    /// In-RAM mirror of the durable topology; authoritative for queries.
    ///
    /// `std::sync::Mutex` is correct in this async store: the critical sections
    /// are synchronous (no `.await` held across the guard), and every disk
    /// access happens *outside* the lock.
    ranges: Mutex<StoredRanges>,
}

impl RocksdbBlockStore {
    /// Open (creating if absent) the store at `path` and load the topology —
    /// from the checkpoint, or by rebuilding it from the block keys if there is
    /// no checkpoint yet.
    pub fn open<P>(path: P) -> AnyResult<Self>
    where
        P: AsRef<Path>,
    {
        let mut db_opts = Options::default();
        db_opts.create_if_missing(true);
        db_opts.create_missing_column_families(true);

        // The default CF carries only the tiny topology checkpoint, so it needs
        // no tuning; the blocks CF carries the large payloads and is tuned for
        // them — see `blocks_cf_options`.
        let db = DB::open_cf_with_opts(&db_opts, path, [
            (CF_DEFAULT, Options::default()),
            (CF_BLOCKS, blocks_cf_options()),
        ])?;

        let ranges = match db.get(TOPOLOGY_KEY)? {
            Some(bytes) => {
                StoredRanges::from_ranges(&borsh::from_slice::<Vec<(u64, u64)>>(&bytes)?)
            },
            None => Self::rebuild_topology(&db)?,
        };

        Ok(Self {
            db,
            ranges: Mutex::new(ranges),
        })
    }

    /// Reconstruct the topology by scanning the block keys — the recovery path
    /// when no checkpoint is present (a fresh DB is empty; a populated one
    /// without a checkpoint is a one-off rebuild).
    fn rebuild_topology(db: &DB) -> AnyResult<StoredRanges> {
        let mut ranges = StoredRanges::default();
        for item in db.iterator_cf(blocks_cf(db), IteratorMode::Start) {
            let (key, _) = item?;
            let bytes: [u8; 8] = key
                .as_ref()
                .try_into()
                .map_err(|_| anyhow!("block key is not 8 bytes: {:?}", key))?;
            ranges.insert(u64::from_be_bytes(bytes));
        }
        Ok(ranges)
    }
}

#[async_trait]
impl BlockStore for RocksdbBlockStore {
    async fn put(&self, height: u64, data: &BlockData) -> AnyResult<Option<u64>> {
        // Single-writer (the coordinator): this read-then-clone is race-free.
        // Compute the would-be next topology on a clone so the RAM frontier is
        // not advanced until the block is durable.
        let (next, advance) = {
            let ranges = self.ranges.lock().unwrap();
            if ranges.contains(height) {
                return Ok(None); // idempotent
            }
            let mut next = ranges.clone();
            let advance = next.insert(height);
            (next, advance)
        };

        // Persist block + topology checkpoint atomically and durably, *before*
        // the advance is exposed — so `h <= frontier ⟹ get(h) = Some` holds even
        // across a crash.
        let mut batch = WriteBatch::default();
        batch.put_cf(
            blocks_cf(&self.db),
            height.to_be_bytes(),
            borsh::to_vec(data)?,
        );
        batch.put(TOPOLOGY_KEY, borsh::to_vec(&next.to_ranges())?);
        self.db.write(batch)?;

        // The block is now durable; commit the advanced topology to RAM.
        *self.ranges.lock().unwrap() = next;
        Ok(advance)
    }

    async fn get(&self, height: u64) -> AnyResult<Option<BlockData>> {
        match self.db.get_cf(blocks_cf(&self.db), height.to_be_bytes())? {
            Some(bytes) => Ok(Some(borsh::from_slice(&bytes)?)),
            None => Ok(None),
        }
    }

    async fn contiguous_frontier(&self) -> AnyResult<Option<u64>> {
        Ok(self.ranges.lock().unwrap().contiguous_top())
    }

    async fn lowest_gap(&self) -> AnyResult<Option<(u64, u64)>> {
        Ok(self.ranges.lock().unwrap().first_gap())
    }
}

/// Handle to the blocks CF — a cheap lookup; the CF always exists once opened.
fn blocks_cf(db: &DB) -> &ColumnFamily {
    db.cf_handle(CF_BLOCKS)
        .expect("blocks column family must exist")
}

/// Tuning for the blocks CF, shaped by the workload: keys are fixed 8-byte
/// big-endian heights, values are large, immutable, compressible [`BlockData`],
/// written append-mostly and never overwritten or deleted. (The fixed key size
/// is not where the wins are — the layout is already ideal for ordered point
/// reads; the tuning targets the large values and the append-only writes.)
fn blocks_cf_options() -> Options {
    let mut opts = Options::default();

    // Key-value separation (BlobDB): the large block payloads live in blob
    // files, leaving the LSM tiny (key → blob pointer). Compaction then does not
    // rewrite the big values — low write-amplification on a multi-million-block
    // backfill — and key scans (e.g. the topology rebuild) stay cheap. Blocks
    // below the threshold stay inline, so small cron-only blocks pay no
    // indirection.
    opts.set_enable_blob_files(true);
    opts.set_min_blob_size(4 * 1024);
    opts.set_blob_compression_type(DBCompressionType::Zstd);

    // SST compression: fast lz4 on the churny upper levels, max-ratio zstd at the
    // stable bottom level where ~all of the archive settles.
    opts.set_compression_type(DBCompressionType::Lz4);
    opts.set_bottommost_compression_type(DBCompressionType::Zstd);

    // Append-only, no deletes: leveled compaction with dynamic level sizing
    // minimizes space amplification.
    opts.set_level_compaction_dynamic_level_bytes(true);

    // Larger memtable → fewer flushes during the backfill burst.
    opts.set_write_buffer_size(64 * 1024 * 1024);

    // Read path: a whole-key bloom for the occasional miss, larger blocks to
    // suit large values, and index/filter kept hot in a dedicated cache.
    let mut block_opts = BlockBasedOptions::default();
    block_opts.set_block_size(16 * 1024);
    block_opts.set_bloom_filter(10.0, true);
    block_opts.set_block_cache(&Cache::new_lru_cache(256 * 1024 * 1024));
    block_opts.set_cache_index_and_filter_blocks(true);
    block_opts.set_pin_l0_filter_and_index_blocks_in_cache(true);
    opts.set_block_based_table_factory(&block_opts);

    opts
}

// ---- tests ----

#[cfg(test)]
mod tests {
    use {super::*, dango_temp_rocksdb::TempDataDir};

    fn block(height: u64) -> BlockData {
        use dango_primitives::{Block, BlockInfo, BlockOutcome, Hash256, Timestamp};
        BlockData {
            block: Block {
                info: BlockInfo {
                    height,
                    timestamp: Timestamp::from_nanos(0),
                    hash: Hash256::ZERO,
                },
                txs: vec![],
            },
            outcome: BlockOutcome {
                height,
                app_hash: Hash256::ZERO,
                cron_outcomes: vec![],
                tx_outcomes: vec![],
            },
        }
    }

    #[tokio::test]
    async fn put_get_roundtrip() {
        let dir = TempDataDir::new("_blockstore_roundtrip");
        let store = RocksdbBlockStore::open(&dir).unwrap();

        store.put(5, &block(5)).await.unwrap();
        assert_eq!(store.get(5).await.unwrap().unwrap().height(), 5);
        assert_eq!(store.get(6).await.unwrap(), None);
    }

    #[tokio::test]
    async fn topology_tracks_frontier_and_gap() {
        let dir = TempDataDir::new("_blockstore_topology");
        let store = RocksdbBlockStore::open(&dir).unwrap();

        for height in 1..=3 {
            store.put(height, &block(height)).await.unwrap();
        }
        assert_eq!(store.contiguous_frontier().await.unwrap(), Some(3));
        assert_eq!(store.lowest_gap().await.unwrap(), None);

        // An island above a gap: frontier unchanged, the hole is reported.
        store.put(5, &block(5)).await.unwrap();
        assert_eq!(store.contiguous_frontier().await.unwrap(), Some(3));
        assert_eq!(store.lowest_gap().await.unwrap(), Some((4, 4)));

        // Fill the hole: the frontier jumps across the island to 5.
        store.put(4, &block(4)).await.unwrap();
        assert_eq!(store.contiguous_frontier().await.unwrap(), Some(5));
        assert_eq!(store.lowest_gap().await.unwrap(), None);
    }

    #[tokio::test]
    async fn put_reports_bulk_advance_and_is_idempotent() {
        let dir = TempDataDir::new("_blockstore_advance");
        let store = RocksdbBlockStore::open(&dir).unwrap();

        assert_eq!(store.put(3, &block(3)).await.unwrap(), None); // island
        assert_eq!(store.put(1, &block(1)).await.unwrap(), Some(1)); // frontier 1
        assert_eq!(store.put(2, &block(2)).await.unwrap(), Some(3)); // bridges → 3
        assert_eq!(store.put(2, &block(2)).await.unwrap(), None); // re-put: no-op
    }

    #[tokio::test]
    async fn survives_reopen_from_checkpoint() {
        let dir = TempDataDir::new("_blockstore_reopen");
        {
            let store = RocksdbBlockStore::open(&dir).unwrap();
            for height in 1..=3 {
                store.put(height, &block(height)).await.unwrap();
            }
            store.put(5, &block(5)).await.unwrap(); // leaves a gap at 4
        }

        // Reopen the same path: the topology comes back from the checkpoint with
        // no scan, and the blocks are intact.
        let store = RocksdbBlockStore::open(&dir).unwrap();
        assert_eq!(store.contiguous_frontier().await.unwrap(), Some(3));
        assert_eq!(store.lowest_gap().await.unwrap(), Some((4, 4)));
        assert_eq!(store.get(2).await.unwrap().unwrap().height(), 2);
        assert_eq!(store.get(5).await.unwrap().unwrap().height(), 5);
    }
}
