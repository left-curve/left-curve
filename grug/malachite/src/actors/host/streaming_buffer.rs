use {
    crate::{
        ctx,
        types::{ProposalFin, ProposalInit},
    },
    malachitebft_core_types::Round,
    malachitebft_engine::util::streaming::{StreamId, StreamMessage},
    malachitebft_sync::PeerId,
    std::{
        cmp::Ordering,
        collections::{BTreeMap, BinaryHeap, HashSet},
    },
};

type Sequence = u64;

#[derive(Clone)]
struct MinSeq<T>(StreamMessage<T>);

impl<T> PartialEq for MinSeq<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0.sequence == other.0.sequence
    }
}

impl<T> Eq for MinSeq<T> {}

impl<T> Ord for MinSeq<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        other.0.sequence.cmp(&self.0.sequence)
    }
}

impl<T> PartialOrd for MinSeq<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone)]
struct MinHeap<T>(BinaryHeap<MinSeq<T>>);

impl<T> Default for MinHeap<T> {
    fn default() -> Self {
        Self(BinaryHeap::new())
    }
}

impl<T> MinHeap<T> {
    fn push(&mut self, msg: StreamMessage<T>) {
        self.0.push(MinSeq(msg));
    }

    fn len(&self) -> usize {
        self.0.len()
    }

    fn drain(&mut self) -> Vec<T> {
        let mut vec = Vec::with_capacity(self.0.len());
        while let Some(MinSeq(msg)) = self.0.pop() {
            if let Some(data) = msg.content.into_data() {
                vec.push(data);
            }
        }
        vec
    }
}

#[derive(Default, Clone)]
struct StreamState {
    buffer: MinHeap<ctx!(ProposalPart)>,
    init_info: Option<ProposalInit>,
    seen_sequences: HashSet<Sequence>,
    total_messages: usize,
    fin_received: bool,
}

impl StreamState {
    fn is_done(&self) -> bool {
        self.init_info.is_some() && self.fin_received && self.buffer.len() == self.total_messages
    }

    fn insert(&mut self, msg: StreamMessage<ctx!(ProposalPart)>) -> Option<ProposalParts> {
        if msg.is_first() {
            self.init_info = msg.content.as_data().and_then(|p| p.as_init()).cloned();
        }

        if msg.is_fin() {
            self.fin_received = true;
            self.total_messages = msg.sequence as usize + 1;
        }

        self.buffer.push(msg);

        if self.is_done() {
            let init_info = self.init_info.take()?;

            Some(ProposalParts {
                height: init_info.height,
                round: init_info.round,
                proposer: init_info.proposer,
                parts: self.buffer.drain(),
            })
        } else {
            None
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProposalParts {
    pub height: ctx!(Height),
    pub round: Round,
    pub proposer: ctx!(Address),
    pub parts: Vec<ctx!(ProposalPart)>,
}

impl ProposalParts {
    pub fn init(&self) -> Option<&ProposalInit> {
        self.parts.iter().find_map(|p| p.as_init())
    }

    pub fn fin(&self) -> Option<&ProposalFin> {
        self.parts.iter().find_map(|p| p.as_fin())
    }
}

#[derive(Default, Clone)]
pub struct PartStreamsMap {
    streams: BTreeMap<(PeerId, StreamId), StreamState>,
}

impl PartStreamsMap {
    pub fn insert(
        &mut self,
        peer_id: PeerId,
        msg: StreamMessage<ctx!(ProposalPart)>,
    ) -> Option<ProposalParts> {
        let stream_id = msg.stream_id.clone();

        let state = self
            .streams
            .entry((peer_id, stream_id.clone()))
            .or_default();

        if !state.seen_sequences.insert(msg.sequence) {
            // We have already seen a message with this sequence number.
            return None;
        }

        let result = state.insert(msg);

        if state.is_done() {
            self.streams.remove(&(peer_id, stream_id));
        }

        result
    }
}
