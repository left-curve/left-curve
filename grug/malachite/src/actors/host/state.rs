use {
    crate::{
        ActorResult,
        actors::host::streaming_buffer::PartStreamsMap,
        context::Context,
        ctx,
        types::{Block, BlockHash, DecidedBlock, ProposalParts},
    },
    grug::{IndexedMap, Map, MockStorage, Storage, UniqueIndex},
    malachitebft_app::{
        consensus::Role,
        streaming::{StreamId, StreamMessage},
        types::ProposedValue,
    },
    malachitebft_core_types::Round,
    malachitebft_engine::consensus::ConsensusRef,
    malachitebft_sync::PeerId,
};

pub type HeightKey = u64;
pub type RoundKey = i64;
pub type StreamIdKey = Vec<u8>;

pub struct State {
    db_storage: Box<dyn Storage>,
    memory_storage: MockStorage,
    consensus: Option<ConsensusRef<Context>>,
    pub height: ctx!(Height),
    pub proposer: Option<ctx!(Address)>,
    pub role: Role,
    pub round: Round,
    streams: PartStreamsMap,
    memory_state: MemoryState,
    db_state: DbState,
}

impl State {
    pub fn new<S: Storage + 'static>(storage: S) -> Self {
        Self {
            db_storage: Box::new(storage),
            consensus: None,
            height: <ctx!(Height)>::new(1),
            round: Round::Nil,
            proposer: None,
            role: Role::None,
            streams: PartStreamsMap::default(),
            memory_state: MemoryState::default(),
            db_state: DbState::default(),
            memory_storage: MockStorage::default(),
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

    pub fn buffer_part(
        &mut self,
        peer_id: PeerId,
        part: StreamMessage<ctx!(ProposalPart)>,
    ) -> Option<ProposalParts> {
        self.streams.insert(peer_id, part)
    }

    pub fn with_memory_storage<C, R>(&self, callback: C) -> R
    where
        C: FnOnce(&dyn Storage, &MemoryState) -> R,
    {
        callback(&self.memory_storage, &self.memory_state)
    }

    pub fn with_memory_storage_mut<C, R>(&mut self, callback: C) -> R
    where
        C: FnOnce(&mut dyn Storage, &MemoryState) -> R,
    {
        callback(&mut self.memory_storage, &self.memory_state)
    }

    pub fn with_db_storage<C, R>(&self, callback: C) -> R
    where
        C: FnOnce(&dyn Storage, &DbState) -> R,
    {
        callback(&self.db_storage, &self.db_state)
    }

    pub fn with_db_storage_mut<C, R>(&mut self, callback: C) -> R
    where
        C: FnOnce(&mut dyn Storage, &DbState) -> R,
    {
        callback(&mut self.db_storage, &self.db_state)
    }
}

// -------------------------------- Memory state -------------------------------

pub struct MemoryState {
    pub parts: IndexedMap<'static, StreamIdKey, ProposalParts, PartsIndexes<'static>>,
}

impl Default for MemoryState {
    fn default() -> Self {
        Self {
            parts: IndexedMap::new("parts", PartsIndexes {
                value_id: UniqueIndex::new(|_, parts| parts.fin.hash, "parts", "parts_value_id"),
            }),
        }
    }
}

#[grug::index_list(StreamIdKey, ProposalParts)]
pub struct PartsIndexes<'a> {
    pub value_id: UniqueIndex<'a, StreamIdKey, ctx!(Value::Id), ProposalParts>,
}

// ---------------------------------- Db state ---------------------------------

pub struct DbState {
    pub decided_block: Map<'static, HeightKey, DecidedBlock>,
    pub undecided_block: Map<'static, (HeightKey, RoundKey, BlockHash), Block>,
    pub undecided_proposals: IndexedMap<
        'static,
        (HeightKey, RoundKey, BlockHash),
        ProposedValue<Context>,
        UndecidedProposalIndexes<'static>,
    >,
}

impl Default for DbState {
    fn default() -> Self {
        Self {
            decided_block: Map::new("decided_block"),
            undecided_block: Map::new("undecided_block"),
            undecided_proposals: IndexedMap::new("undecided_proposal", UndecidedProposalIndexes {
                proposer: UniqueIndex::new(
                    |(height, round, ..), value| (*height, *round, value.proposer),
                    "undecided_proposal",
                    "undecided_proposal__proposer",
                ),
            }),
        }
    }
}

#[grug::index_list((HeightKey, RoundKey, BlockHash), ProposedValue<Context>)]
pub struct UndecidedProposalIndexes<'a> {
    pub proposer: UniqueIndex<
        'a,
        (HeightKey, RoundKey, BlockHash),
        (HeightKey, RoundKey, ctx!(Address)),
        ProposedValue<Context>,
    >,
}
