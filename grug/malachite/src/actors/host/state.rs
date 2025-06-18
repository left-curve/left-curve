use {
    crate::{
        ActorResult, HostConfig, RawTx,
        app::HostAppRef,
        context::Context,
        ctx,
        types::{Block, BlockHash, DecidedBlock},
    },
    grug::{Batch, IndexedMap, Map, Storage, UniqueIndex},
    grug_app::{AppResult, ConsensusStorage},
    malachitebft_app::{consensus::Role, streaming::StreamId, types::ProposedValue},
    malachitebft_core_types::Round,
    malachitebft_engine::consensus::ConsensusRef,
    std::time::{Duration, Instant},
};

pub type HeightKey = u64;
pub type RoundKey = i64;

#[derive(Clone)]
pub struct State {
    pub height: ctx!(Height),
    pub proposer: Option<ctx!(Address)>,
    pub role: Role,
    pub round: Round,
    pub config: HostConfig,
    db_storage: Box<dyn ConsensusStorage>,
    consensus: Option<ConsensusRef<Context>>,
    started_round: Instant,
    app: HostAppRef,
    pending_commit_block_hash: Option<BlockHash>,
}

impl State {
    pub fn new<S: ConsensusStorage + 'static>(
        storage: S,
        config: HostConfig,
        app: HostAppRef,
    ) -> Self {
        Self {
            db_storage: Box::new(storage),
            consensus: None,
            height: <ctx!(Height)>::new(1),
            round: Round::Nil,
            proposer: None,
            role: Role::None,
            config,
            started_round: Instant::now(),
            app,
            pending_commit_block_hash: None,
        }
    }

    pub fn set_consensus(&mut self, consensus: ConsensusRef<Context>) {
        self.consensus = Some(consensus);
    }

    pub fn consensus(&self) -> ActorResult<ConsensusRef<Context>> {
        self.consensus
            .clone()
            .ok_or(anyhow::anyhow!("Consensus not set").into())
    }

    pub fn stream_id(&self) -> StreamId {
        let mut bytes = Vec::with_capacity(size_of::<u64>() + size_of::<u32>());
        bytes.extend_from_slice(&self.height.to_be_bytes());
        // TODO: can this panic?
        bytes.extend_from_slice(&self.round.as_u32().unwrap().to_be_bytes());
        StreamId::new(bytes.into())
    }

    pub fn started_round(
        &mut self,
        height: ctx!(Height),
        round: Round,
        proposer: ctx!(Address),
        role: Role,
    ) {
        self.started_round = Instant::now();
        self.height = height;
        self.round = round;
        self.proposer = Some(proposer);
        self.role = role;
    }

    pub fn calculate_block_sleep(&self) -> Duration {
        let diff = self.started_round.elapsed();
        if diff < self.config.block_time {
            self.config.block_time - diff
        } else {
            Duration::ZERO
        }
    }

    pub fn prepare_proposal(&self, txs: Vec<RawTx>) -> Vec<RawTx> {
        self.app
            .prepare_proposal(txs, self.config.max_tx_bytes.as_u64() as usize)
    }

    pub fn finalize_block<T>(&mut self, block: &Block<T>) -> AppResult<BlockHash> {
        let app_hash = self.app.finalize_block(block.as_block_info(), &block.txs)?;

        let block_hash = block.calculate_block_hash(app_hash);

        self.pending_commit_block_hash = Some(block_hash);

        Ok(block_hash)
    }

    pub fn commit(&mut self, block: &Block, consensus_batch: Batch) -> AppResult<()> {
        let mut rerun = true;

        // If the pending_commit_block_hash is equal to the block hash, this mean that the app batch in memory
        // is the same as the block we are trying to commit, so we don't need to rerun the finalize_block.
        if let Some(pending) = self.pending_commit_block_hash {
            if pending == block.block_hash() {
                rerun = false;
            }
        }

        if rerun {
            let block_hash = self.finalize_block(block)?;
            if block_hash != block.block_hash() {
                // TODO: We should panic here? This mean the the we have some non-deterministic behavior in the app.
                panic!("Block hash mismatch");
            }
        }

        self.app.commit(consensus_batch)?;
        self.pending_commit_block_hash = None;

        Ok(())
    }
}

impl Storage for State {
    fn read(&self, key: &[u8]) -> Option<Vec<u8>> {
        self.db_storage.read(key)
    }

    fn scan<'a>(
        &'a self,
        min: Option<&[u8]>,
        max: Option<&[u8]>,
        order: grug::Order,
    ) -> Box<dyn Iterator<Item = grug::Record> + 'a> {
        self.db_storage.scan(min, max, order)
    }

    fn scan_keys<'a>(
        &'a self,
        min: Option<&[u8]>,
        max: Option<&[u8]>,
        order: grug::Order,
    ) -> Box<dyn Iterator<Item = Vec<u8>> + 'a> {
        self.db_storage.scan_keys(min, max, order)
    }

    fn scan_values<'a>(
        &'a self,
        min: Option<&[u8]>,
        max: Option<&[u8]>,
        order: grug::Order,
    ) -> Box<dyn Iterator<Item = Vec<u8>> + 'a> {
        self.db_storage.scan_values(min, max, order)
    }

    fn write(&mut self, key: &[u8], value: &[u8]) {
        self.db_storage.write(key, value)
    }

    fn remove(&mut self, key: &[u8]) {
        self.db_storage.remove(key)
    }

    fn remove_range(&mut self, min: Option<&[u8]>, max: Option<&[u8]>) {
        self.db_storage.remove_range(min, max)
    }
}

// ---------------------------------- Db state ---------------------------------

pub const DECIDED_BLOCK: Map<'static, HeightKey, DecidedBlock> = Map::new("decided_block");

pub const UNDECIDED_BLOCK: Map<'static, BlockHash, Block> = Map::new("undecided_block");

pub const UNDECIDED_PROPOSALS: IndexedMap<
    'static,
    (HeightKey, RoundKey, BlockHash),
    ProposedValue<Context>,
    UndecidedProposalIndexes<'static>,
> = IndexedMap::new("undecided_proposal", UndecidedProposalIndexes {
    proposer: UniqueIndex::new(
        |(height, round, ..), value| (*height, *round, value.proposer),
        "undecided_proposal",
        "undecided_proposal__proposer",
    ),
});

#[grug::index_list((HeightKey, RoundKey, BlockHash), ProposedValue<Context>)]
pub struct UndecidedProposalIndexes<'a> {
    pub proposer: UniqueIndex<
        'a,
        (HeightKey, RoundKey, BlockHash),
        (HeightKey, RoundKey, ctx!(Address)),
        ProposedValue<Context>,
    >,
}

pub fn latest_height(consensus_storage: &dyn ConsensusStorage) -> Option<HeightKey> {
    DECIDED_BLOCK
        .range(consensus_storage, None, None, grug::Order::Descending)
        .next()
        .map(|res| res.unwrap().0)
}
