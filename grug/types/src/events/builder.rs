use {
    crate::{ContractEvent, EventName, StdResult},
    serde::Serialize,
};

/// A helper that provides better looking syntax for building a list of events.
#[derive(Default)]
pub struct EventBuilder {
    events: Vec<ContractEvent>,
}

impl EventBuilder {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            events: Vec::with_capacity(capacity),
        }
    }

    pub fn push<E>(&mut self, event: E) -> StdResult<&mut Self>
    where
        E: EventName + Serialize,
    {
        self.events.push(ContractEvent::new(&event)?);
        Ok(self)
    }

    pub fn may_push<E>(&mut self, maybe_event: Option<E>) -> StdResult<&mut Self>
    where
        E: EventName + Serialize,
    {
        if let Some(event) = maybe_event {
            self.events.push(ContractEvent::new(&event)?);
        }

        Ok(self)
    }
}

impl IntoIterator for EventBuilder {
    type IntoIter = std::vec::IntoIter<ContractEvent>;
    type Item = ContractEvent;

    fn into_iter(self) -> Self::IntoIter {
        self.events.into_iter()
    }
}
