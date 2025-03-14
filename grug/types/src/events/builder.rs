use crate::ContractEvent;

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

    pub fn push<T>(&mut self, event: T) -> Result<&mut Self, T::Error>
    where
        T: TryInto<ContractEvent>,
    {
        self.events.push(event.try_into()?);
        Ok(self)
    }

    pub fn may_push<T>(&mut self, maybe_event: Option<T>) -> Result<&mut Self, T::Error>
    where
        T: TryInto<ContractEvent>,
    {
        if let Some(event) = maybe_event {
            self.events.push(event.try_into()?);
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
