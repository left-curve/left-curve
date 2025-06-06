use {
    crate::{
        ActorResult,
        actors::host::streaming_buffer::{PartStreamsMap, ProposalParts},
        context::Context,
        ctx,
        types::{ProposalFin, ProposalInit, ProposalPart, RawTx},
    },
    grug::{Hash256, IndexedMap, Map, MockStorage, Storage, UniqueIndex},
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

const BLOCKS: Map<HeightKey, ()> = Map::new("block");

const PARTS: IndexedMap<StreamIdKey, StoreParts, PartsIndexes> =
    IndexedMap::new("parts", PartsIndexes {
        value_id: UniqueIndex::new(|_, parts| parts.fin.hash, "parts", "parts_value_id"),
    });

#[grug::index_list(StreamIdKey, StoreParts)]
pub struct PartsIndexes<'a> {
    pub value_id: UniqueIndex<'a, StreamIdKey, ctx!(Value::Id), StoreParts>,
}

const UNDECIDED_PROPOSALS: IndexedMap<
    (HeightKey, RoundKey, Hash256),
    ProposedValue<Context>,
    RoundsIndexes,
> = IndexedMap::new("undecided_proposal", RoundsIndexes {
    proposer: UniqueIndex::new(
        |(height, round, ..), value| (*height, *round, value.proposer),
        "undecided_proposal",
        "undecided_proposal__proposer",
    ),
});

#[grug::index_list((HeightKey, RoundKey, Hash256), ProposedValue<Context>)]
pub struct RoundsIndexes<'a> {
    pub proposer: UniqueIndex<
        'a,
        (HeightKey, RoundKey, Hash256),
        (HeightKey, RoundKey, ctx!(Address)),
        ProposedValue<Context>,
    >,
}

#[grug::derive(Borsh)]
pub struct StoreParts {
    pub init: ProposalInit,
    pub data: Vec<RawTx>,
    pub fin: ProposalFin,
}

impl StoreParts {
    pub fn new(init: ProposalInit, data: Vec<RawTx>, fin: ProposalFin) -> Self {
        Self { init, data, fin }
    }
}

impl IntoIterator for StoreParts {
    type IntoIter = <Vec<ProposalPart> as IntoIterator>::IntoIter;
    type Item = ProposalPart;

    fn into_iter(self) -> Self::IntoIter {
        vec![
            ProposalPart::Init(self.init),
            ProposalPart::Data(self.data),
            ProposalPart::Fin(self.fin),
        ]
        .into_iter()
    }
}

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

    pub fn add_part(
        &mut self,
        peer_id: PeerId,
        part: StreamMessage<ctx!(ProposalPart)>,
    ) -> Option<ProposalParts> {
        self.streams.insert(peer_id, part)
    }

    pub fn with_memory_storage<C, R>(&self, callback: C) -> R
    where
        C: Fn(&dyn Storage, &MemoryState) -> R,
    {
        callback(&self.memory_storage, &self.memory_state)
    }

    pub fn with_memory_storage_mut<C, R>(&mut self, callback: C) -> R
    where
        C: Fn(&mut dyn Storage, &MemoryState) -> R,
    {
        callback(&mut self.memory_storage, &self.memory_state)
    }

    pub fn with_db_storage<C, R>(&self, callback: C) -> R
    where
        C: Fn(&dyn Storage, &DbState) -> R,
    {
        callback(&self.db_storage, &self.db_state)
    }

    pub fn with_db_storage_mut<C, R>(&mut self, callback: C) -> R
    where
        C: Fn(&mut dyn Storage, &DbState) -> R,
    {
        callback(&mut self.db_storage, &self.db_state)
    }
}

pub struct MemoryState {
    pub parts: IndexedMap<'static, StreamIdKey, StoreParts, PartsIndexes<'static>>,
}

impl Default for MemoryState {
    fn default() -> Self {
        Self { parts: PARTS }
    }
}

pub struct DbState {
    pub blocks: Map<'static, HeightKey, ()>,
    pub undecided_proposals: IndexedMap<
        'static,
        (HeightKey, RoundKey, Hash256),
        ProposedValue<Context>,
        RoundsIndexes<'static>,
    >,
}

impl Default for DbState {
    fn default() -> Self {
        Self {
            blocks: BLOCKS,
            undecided_proposals: UNDECIDED_PROPOSALS,
        }
    }
}
