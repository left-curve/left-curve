use {
    super::{
        flatten_commitment_status, flatten_tx_events, EventId, EvtConfigure, FlatCategory,
        FlatCommitmentStatus, FlatEvent, FlatEventInfo, FlatEventStatus,
        FlatEventStatusDiscriminants, FlatEvtAuthenticate, FlatEvtBackrun, FlatEvtCron,
        FlatEvtExecute, FlatEvtFinalize, FlatEvtGuest, FlatEvtInstantiate, FlatEvtMigrate,
        FlatEvtReply, FlatEvtTransfer, FlatEvtWithhold, FlattenStatus,
    },
    crate::{
        BlockOutcome, CommitmentStatus, ContractEvent, Defined, EvtUpload, MaybeDefined, TxEvents,
        Undefined,
    },
    std::marker::PhantomData,
};

// -------------------------------- EventFilter --------------------------------

pub struct EventFilter<
    F,
    ES = Undefined<FlatEventStatusDiscriminants>,
    CS = Undefined<FlatCommitmentStatus>,
    P = Undefined<Box<dyn Fn(&F) -> bool>>,
> where
    FlatEvent: AsVariant<F>,
{
    _filter: PhantomData<F>,
    event_status: ES,
    commitment_status: CS,
    predicate: P,
    events: Vec<FlatEventInfo>,
}

impl<F, CS, P> EventFilter<F, Undefined<FlatEventStatusDiscriminants>, CS, P>
where
    FlatEvent: AsVariant<F>,
{
    /// Filter events by event status via [`FlatEventStatusDiscriminants`].
    pub fn with_event_status(
        self,
        event_status: FlatEventStatusDiscriminants,
    ) -> EventFilter<F, Defined<FlatEventStatusDiscriminants>, CS, P> {
        EventFilter {
            _filter: PhantomData,
            event_status: Defined::new(event_status),
            commitment_status: self.commitment_status,
            predicate: self.predicate,
            events: self.events,
        }
    }
}

impl<F, ES, P> EventFilter<F, ES, Undefined<FlatCommitmentStatus>, P>
where
    FlatEvent: AsVariant<F>,
{
    /// Filter events by commitment status via [`FlatCommitmentStatus`].
    pub fn with_commitment_status(
        self,
        commitment_status: FlatCommitmentStatus,
    ) -> EventFilter<F, ES, Defined<FlatCommitmentStatus>, P> {
        EventFilter {
            _filter: PhantomData,
            event_status: self.event_status,
            commitment_status: Defined::new(commitment_status),
            predicate: self.predicate,
            events: self.events,
        }
    }
}

impl<F, ES, CS> EventFilter<F, ES, CS, Undefined<Box<dyn Fn(&F) -> bool>>>
where
    FlatEvent: AsVariant<F>,
{
    /// Filter events by a predicate.
    pub fn with_predicate<P>(
        self,
        predicate: P,
    ) -> EventFilter<F, ES, CS, Defined<Box<dyn Fn(&F) -> bool>>>
    where
        P: Fn(&F) -> bool + 'static,
    {
        EventFilter {
            _filter: PhantomData,
            event_status: self.event_status,
            commitment_status: self.commitment_status,
            predicate: Defined::new(Box::new(predicate)),
            events: self.events,
        }
    }
}

impl<F> EventFilter<F>
where
    FlatEvent: AsVariant<F>,
{
    pub fn new(events: Vec<FlatEventInfo>) -> Self {
        Self {
            _filter: PhantomData,
            event_status: Undefined::new(),
            commitment_status: Undefined::new(),
            predicate: Undefined::new(),
            events,
        }
    }
}

impl<F, ES, CS, P> EventFilter<F, ES, CS, P>
where
    FlatEvent: AsVariant<F>,
    ES: MaybeDefined<FlatEventStatusDiscriminants>,
    CS: MaybeDefined<FlatCommitmentStatus>,
    P: MaybeDefined<Box<dyn Fn(&F) -> bool>>,
{
    /// Takes the events that match the filter.
    pub fn take(self) -> FilterResult<FilteredEvent<F>> {
        let events = self
            .events
            .into_iter()
            .filter_map(|event| {
                if let Some(event_status) = self.event_status.maybe_inner() {
                    if FlatEventStatusDiscriminants::from(event.event_status.clone())
                        != *event_status
                    {
                        return None;
                    }
                }

                if let Some(commitment_status) = self.commitment_status.maybe_inner() {
                    if event.commitment_status != *commitment_status {
                        return None;
                    }
                }

                let maybe_event = event.event.maybe_variant();

                if let (Some(event), Some(predicate)) = (&maybe_event, self.predicate.maybe_inner())
                {
                    if !predicate(event) {
                        return None;
                    }
                }

                maybe_event.map(|typed_event| FilteredEvent {
                    commitment_status: event.commitment_status,
                    event_status: event.event_status,
                    event: typed_event,
                })
            })
            .collect();

        FilterResult { events }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilteredEvent<T> {
    pub commitment_status: FlatCommitmentStatus,
    pub event_status: FlatEventStatus,
    pub event: T,
}

pub struct FilterResult<T> {
    events: Vec<T>,
}

impl<T> FilterResult<T> {
    /// Asserts that there is exactly one event and returns it.
    pub fn one(self) -> T {
        assert_eq!(self.events.len(), 1);
        self.events.into_iter().next().unwrap()
    }

    /// Returns the events as a vector.
    pub fn all(self) -> Vec<T> {
        self.events
    }

    /// Asserts the number of events and returns them as fixed-size array.
    pub fn exact<const N: usize>(self) -> [T; N]
    where
        T: std::fmt::Debug,
    {
        self.events.try_into().unwrap()
    }
}

// -------------------------------- SearchEvent --------------------------------

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

// --------------------------------- AsVariant ---------------------------------

/// Trait that allows to convert an Enum to inner value of a specific variant.
pub trait AsVariant<V> {
    fn maybe_variant(self) -> Option<V>;
}

macro_rules! impl_as_variant {
    ( $enum:ident, $($variant:ident => $flat_variant:ident),*) => {
        $(impl AsVariant<$flat_variant> for $enum {
            fn maybe_variant(self) -> Option<$flat_variant> {
                if let $enum::$variant(inner) = self {
                    Some(inner)
                } else {
                    None
                }
            }
        })*
    };
}

impl_as_variant!(
    FlatEvent,
    Configure     => EvtConfigure,
    Transfer      => FlatEvtTransfer,
    Upload        => EvtUpload,
    Instantiate   => FlatEvtInstantiate,
    Execute       => FlatEvtExecute,
    Migrate       => FlatEvtMigrate,
    Reply         => FlatEvtReply,
    Authenticate  => FlatEvtAuthenticate,
    Backrun       => FlatEvtBackrun,
    Withhold      => FlatEvtWithhold,
    Finalize      => FlatEvtFinalize,
    Cron          => FlatEvtCron,
    Guest         => FlatEvtGuest,
    ContractEvent => ContractEvent
);
