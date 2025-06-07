use {
    crate::{
        ctx,
        types::{ProposalFin, ProposalInit, ProposalPart, ProposalParts, RawTx},
    },
    malachitebft_app::streaming::StreamContent,
    malachitebft_engine::util::streaming::{StreamId, StreamMessage},
    malachitebft_sync::PeerId,
    std::collections::BTreeMap,
};

#[derive(Default)]
pub struct StreamParts {
    pub init: Option<ProposalInit>,
    pub data: Option<Vec<RawTx>>,
    pub fin: Option<ProposalFin>,
    pub stop: bool,
}

impl StreamParts {
    pub fn is_done(&self) -> bool {
        self.init.is_some() && self.data.is_some() && self.fin.is_some() && self.stop
    }

    pub fn take(self) -> ProposalParts {
        ProposalParts {
            init: self.init.unwrap(),
            data: self.data.unwrap(),
            fin: self.fin.unwrap(),
        }
    }
}

#[derive(Default)]
pub struct PartStreamsMap {
    streams: BTreeMap<(PeerId, StreamId), StreamParts>,
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

        match msg.content {
            StreamContent::Data(part) => match part {
                ProposalPart::Init(proposal_init) => {
                    if msg.sequence == 0 && state.init.is_none() {
                        state.init = Some(proposal_init);
                    }
                },
                ProposalPart::Data(items) => {
                    if msg.sequence == 1 && state.data.is_none() {
                        state.data = Some(items);
                    }
                },
                ProposalPart::Fin(proposal_fin) => {
                    if msg.sequence == 2 && state.fin.is_none() {
                        state.fin = Some(proposal_fin);
                    }
                },
            },
            StreamContent::Fin => {
                if msg.sequence == 3 && !state.stop {
                    state.stop = true;
                }
            },
        }

        if state.is_done() {
            let state = self.streams.remove(&(peer_id, stream_id));
            state.map(|state| state.take())
        } else {
            None
        }
    }
}
