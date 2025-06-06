use {
    crate::{ActorResult, context::Context, ctx, types::ProposalPart},
    grug::{Hash256, IndexedMap, Map, Storage, UniqueIndex},
    malachitebft_app::{consensus::Role, streaming::StreamId, types::ProposedValue},
    malachitebft_core_types::Round,
    malachitebft_engine::consensus::ConsensusRef,
};

pub type HeightKey = u64;
pub type RoundKey = i64;
pub type StreamIdKey<'a> = &'a [u8];

pub const BLOCKS: Map<HeightKey, ()> = Map::new("block");

pub const PARTS: Map<StreamIdKey, Vec<ProposalPart>> = Map::new("parts");

pub const ROUNDS: IndexedMap<
    (HeightKey, RoundKey, Hash256),
    ProposedValue<Context>,
    RoundsIndexes,
> = IndexedMap::new("round", RoundsIndexes {
    proposer: UniqueIndex::new(
        |(height, round, ..), value| (*height, *round, value.proposer),
        "round",
        "round_proposer",
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

#[derive(Clone)]
pub struct State {
    storage: Box<dyn Storage>,
    consensus: Option<ConsensusRef<Context>>,
    pub height: ctx!(Height),
    pub proposer: Option<ctx!(Address)>,
    pub role: Role,
    pub round: Round,
}

impl State {
    pub fn new<S: Storage + 'static>(storage: S) -> Self {
        Self {
            storage: Box::new(storage),
            consensus: None,
            height: <ctx!(Height)>::new(1),
            round: Round::Nil,
            proposer: None,
            role: Role::None,
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
}

impl Storage for State {
    fn read(&self, key: &[u8]) -> Option<Vec<u8>> {
        self.storage.read(key)
    }

    fn scan<'a>(
        &'a self,
        min: Option<&[u8]>,
        max: Option<&[u8]>,
        order: grug::Order,
    ) -> Box<dyn Iterator<Item = grug::Record> + 'a> {
        self.storage.scan(min, max, order)
    }

    fn scan_keys<'a>(
        &'a self,
        min: Option<&[u8]>,
        max: Option<&[u8]>,
        order: grug::Order,
    ) -> Box<dyn Iterator<Item = Vec<u8>> + 'a> {
        self.storage.scan_keys(min, max, order)
    }

    fn scan_values<'a>(
        &'a self,
        min: Option<&[u8]>,
        max: Option<&[u8]>,
        order: grug::Order,
    ) -> Box<dyn Iterator<Item = Vec<u8>> + 'a> {
        self.storage.scan_values(min, max, order)
    }

    fn write(&mut self, key: &[u8], value: &[u8]) {
        self.storage.write(key, value)
    }

    fn remove(&mut self, key: &[u8]) {
        self.storage.remove(key)
    }

    fn remove_range(&mut self, min: Option<&[u8]>, max: Option<&[u8]>) {
        self.storage.remove_range(min, max)
    }
}
