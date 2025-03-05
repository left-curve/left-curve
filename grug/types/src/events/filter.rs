use {
    super::EventId,
    crate::{
        AsVariant, Defined, FlatCommitmentStatus, FlatEvent, FlatEventInfo, FlatEventStatus,
        FlatEventStatusDiscriminants, MaybeDefined, Undefined,
    },
    std::{fmt::Debug, marker::PhantomData},
};

pub struct EventFilter<
    F,
    ES = Undefined<FlatEventStatusDiscriminants>,
    CS = Undefined<FlatCommitmentStatus>,
    P = Undefined<Box<dyn Fn(&F) -> bool>>,
> where
    FlatEvent: AsVariant<F>,
{
    events: Vec<FlatEventInfo>,
    event_status: ES,
    commitment_status: CS,
    predicate: P,
    _filter: PhantomData<F>,
}

impl<F> EventFilter<F>
where
    FlatEvent: AsVariant<F>,
{
    pub fn new(events: Vec<FlatEventInfo>) -> Self {
        Self {
            events,
            event_status: Undefined::new(),
            commitment_status: Undefined::new(),
            predicate: Undefined::new(),
            _filter: PhantomData,
        }
    }
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

impl<F, ES, CS, P> EventFilter<F, ES, CS, P>
where
    FlatEvent: AsVariant<F>,
    ES: MaybeDefined<FlatEventStatusDiscriminants>,
    CS: MaybeDefined<FlatCommitmentStatus>,
    P: MaybeDefined<Box<dyn Fn(&F) -> bool>>,
{
    /// Takes the events that match the filter.
    pub fn take(self) -> FilterResult<FilteredEvent<F>> {
        let events: Vec<FilteredEvent<F>> = self
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
                    id: event.id,
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
    pub id: EventId,
    pub commitment_status: FlatCommitmentStatus,
    pub event_status: FlatEventStatus,
    pub event: T,
}

pub struct FilterResult<T> {
    events: Vec<T>,
}

impl<T> FilterResult<T> {
    /// Returns the events as a vector.
    pub fn all(self) -> Vec<T> {
        self.events
    }

    /// Asserts that there is exactly one event and returns it.
    pub fn one(self) -> T {
        assert_eq!(
            self.events.len(),
            1,
            "expecting exactly one event, got: {}",
            self.events.len()
        );

        self.events.into_iter().next().unwrap()
    }

    /// Asserts the number of events and returns them as fixed-size array.
    pub fn exact<const N: usize>(self) -> [T; N]
    where
        T: Debug,
    {
        assert_eq!(
            self.events.len(),
            N,
            "expecting exactly {} events, got: {}",
            N,
            self.events.len()
        );

        self.events.try_into().unwrap()
    }
}
