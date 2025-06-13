use {
    crate::{
        ActorResult, HostConfig,
        context::Context,
        ctx,
        types::{Block, BlockHash, DecidedBlock},
    },
    grug::{IndexedMap, Map, Storage, UniqueIndex},
    grug_app::ConsensusStorage,
    malachitebft_app::{consensus::Role, streaming::StreamId, types::ProposedValue},
    malachitebft_core_types::Round,
    malachitebft_engine::consensus::ConsensusRef,
    std::time::{Duration, Instant},
};

pub type HeightKey = u64;
pub type RoundKey = i64;

#[derive(Clone)]
pub struct State {
    pub db_storage: Box<dyn ConsensusStorage>,
    consensus: Option<ConsensusRef<Context>>,
    pub height: ctx!(Height),
    pub proposer: Option<ctx!(Address)>,
    pub role: Role,
    pub round: Round,
    config: HostConfig,
    started_round: Instant,
}

impl State {
    pub fn new<S: ConsensusStorage + 'static>(storage: S, config: HostConfig) -> Self {
        Self {
            db_storage: Box::new(storage),
            consensus: None,
            height: <ctx!(Height)>::new(1),
            round: Round::Nil,
            proposer: None,
            role: Role::None,
            config,
            started_round: Instant::now(),
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

    pub fn started_round(&mut self) {
        self.started_round = Instant::now();
    }

    pub fn calculate_block_sleep(&self) -> Duration {
        let diff = self.started_round.elapsed();
        if diff < self.config.block_time {
            self.config.block_time - diff
        } else {
            Duration::ZERO
        }
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
