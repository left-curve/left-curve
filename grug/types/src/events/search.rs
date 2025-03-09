use crate::{
    AsVariant, BlockOutcome, CommitmentStatus, EventFilter, EventId, FlatCategory, FlatEvent,
    FlatEventInfo, FlattenStatus, TxEvents, flatten_commitment_status, flatten_tx_events,
};

pub trait SearchEvent: Sized {
    /// Create a [`EventFilter`] to search for specific events.
    fn search_event<F>(self) -> EventFilter<F>
    where
        FlatEvent: AsVariant<F>,
    {
        EventFilter::new(self.flat())
    }

    fn flat(self) -> Vec<FlatEventInfo>;
}

impl SearchEvent for TxEvents {
    fn flat(self) -> Vec<FlatEventInfo> {
        flatten_tx_events(self, 0, 0)
    }
}

impl<T> SearchEvent for CommitmentStatus<T>
where
    T: FlattenStatus,
{
    fn flat(self) -> Vec<FlatEventInfo> {
        flatten_commitment_status(&mut EventId::new(0, FlatCategory::Tx, 0, 0), self)
    }
}

impl SearchEvent for BlockOutcome {
    fn flat(self) -> Vec<FlatEventInfo> {
        self.tx_outcomes
            .into_iter()
            .fold(vec![], |mut acc, tx_outcome| {
                acc.extend(tx_outcome.events.flat());
                acc
            })
    }
}
