use {
    crate::{
        Addr, CheckedContractEvent, CommitmentStatus, Event, EventStatus, EvtAuthenticate,
        EvtBackrun, EvtConfigure, EvtExecute, EvtFinalize, EvtGuest, EvtInstantiate, EvtMigrate,
        EvtReply, EvtTransfer, EvtUpload, EvtWithhold, MsgsAndBackrunEvents, SubEvent,
        SubEventStatus, TxEvents, TxOutcome,
    },
    grug_math::Inner,
    serde_json::Value as JsonValue,
    std::{collections::HashSet, str::FromStr},
};

/// Describes an object that can be searched for addresses relevant to it.
///
/// This is intended for indexing transactions (txs). Our block explorer should
/// display all txs related to a given address. This doesn't only include all
/// all transactions sent by this address, but also txs where this address is
/// involved in any way, e.g. it appears in any event.
///
/// This trait is implemented for [`TxOutcome`], such that for any given tx
/// outcome, the indexer can find all addresses involved in it, and index appropriately.
pub trait AddressSearcher {
    fn search_for_addresses(&self, addresses: &mut HashSet<Addr>);
}

impl AddressSearcher for TxOutcome {
    fn search_for_addresses(&self, addresses: &mut HashSet<Addr>) {
        self.events.search_for_addresses(addresses);
    }
}

impl AddressSearcher for TxEvents {
    fn search_for_addresses(&self, addresses: &mut HashSet<Addr>) {
        self.withhold.search_for_addresses(addresses);
        self.authenticate.search_for_addresses(addresses);
        self.msgs_and_backrun.search_for_addresses(addresses);
        self.finalize.search_for_addresses(addresses);
    }
}

impl AddressSearcher for MsgsAndBackrunEvents {
    fn search_for_addresses(&self, addresses: &mut HashSet<Addr>) {
        for event in &self.msgs {
            event.search_for_addresses(addresses);
        }

        self.backrun.search_for_addresses(addresses);
    }
}

impl AddressSearcher for Event {
    fn search_for_addresses(&self, addresses: &mut HashSet<Addr>) {
        match self {
            Event::Configure(event) => event.search_for_addresses(addresses),
            Event::Transfer(event) => event.search_for_addresses(addresses),
            Event::Upload(event) => event.search_for_addresses(addresses),
            Event::Instantiate(event) => event.search_for_addresses(addresses),
            Event::Execute(event) => event.search_for_addresses(addresses),
            Event::Migrate(event) => event.search_for_addresses(addresses),
            Event::Reply(event) => event.search_for_addresses(addresses),
            Event::Authenticate(event) => event.search_for_addresses(addresses),
            Event::Backrun(event) => event.search_for_addresses(addresses),
            Event::Withhold(event) => event.search_for_addresses(addresses),
            Event::Finalize(event) => event.search_for_addresses(addresses),
            // `EvtCron` never appears in a transaction, so no need to search.
            Event::Cron(_) => {},
        }
    }
}

impl AddressSearcher for EvtConfigure {
    fn search_for_addresses(&self, addresses: &mut HashSet<Addr>) {
        addresses.insert(self.sender);
    }
}

impl AddressSearcher for EvtTransfer {
    fn search_for_addresses(&self, addresses: &mut HashSet<Addr>) {
        addresses.insert(self.sender);

        for recipient in self.transfers.keys() {
            addresses.insert(*recipient);
        }

        self.bank_guest.search_for_addresses(addresses);

        for (guest, event) in &self.receive_guests {
            addresses.insert(*guest);
            event.search_for_addresses(addresses);
        }
    }
}

impl AddressSearcher for EvtUpload {
    fn search_for_addresses(&self, addresses: &mut HashSet<Addr>) {
        addresses.insert(self.sender);
    }
}

impl AddressSearcher for EvtInstantiate {
    fn search_for_addresses(&self, addresses: &mut HashSet<Addr>) {
        addresses.insert(self.sender);
        addresses.insert(self.contract);

        if let Some(admin) = self.admin {
            addresses.insert(admin);
        }

        self.transfer_event.search_for_addresses(addresses);
        self.guest_event.search_for_addresses(addresses);
    }
}

impl AddressSearcher for EvtExecute {
    fn search_for_addresses(&self, addresses: &mut HashSet<Addr>) {
        addresses.insert(self.sender);
        addresses.insert(self.contract);

        self.transfer_event.search_for_addresses(addresses);
        self.guest_event.search_for_addresses(addresses);
    }
}

impl AddressSearcher for EvtMigrate {
    fn search_for_addresses(&self, addresses: &mut HashSet<Addr>) {
        addresses.insert(self.sender);
        addresses.insert(self.contract);

        self.guest_event.search_for_addresses(addresses);
    }
}

impl AddressSearcher for EvtReply {
    fn search_for_addresses(&self, addresses: &mut HashSet<Addr>) {
        addresses.insert(self.contract);

        self.guest_event.search_for_addresses(addresses);
    }
}

impl AddressSearcher for EvtAuthenticate {
    fn search_for_addresses(&self, addresses: &mut HashSet<Addr>) {
        addresses.insert(self.sender);

        self.guest_event.search_for_addresses(addresses);
    }
}

impl AddressSearcher for EvtBackrun {
    fn search_for_addresses(&self, addresses: &mut HashSet<Addr>) {
        addresses.insert(self.sender);

        self.guest_event.search_for_addresses(addresses);
    }
}

impl AddressSearcher for EvtWithhold {
    fn search_for_addresses(&self, addresses: &mut HashSet<Addr>) {
        addresses.insert(self.sender);

        self.guest_event.search_for_addresses(addresses);
    }
}

impl AddressSearcher for EvtFinalize {
    fn search_for_addresses(&self, addresses: &mut HashSet<Addr>) {
        addresses.insert(self.sender);

        self.guest_event.search_for_addresses(addresses);
    }
}

impl AddressSearcher for EvtGuest {
    fn search_for_addresses(&self, addresses: &mut HashSet<Addr>) {
        addresses.insert(self.contract);

        for event in &self.contract_events {
            event.search_for_addresses(addresses);
        }

        for event in &self.sub_events {
            event.search_for_addresses(addresses);
        }
    }
}

impl<T> AddressSearcher for CommitmentStatus<T>
where
    T: AddressSearcher,
{
    fn search_for_addresses(&self, addresses: &mut HashSet<Addr>) {
        match self {
            CommitmentStatus::Committed(event) | CommitmentStatus::Failed { event, .. } => {
                event.search_for_addresses(addresses)
            },
            // TODO: should we find addresses in `Reverted`???
            CommitmentStatus::Reverted { .. } | CommitmentStatus::NotReached => {},
        }
    }
}

impl<T> AddressSearcher for EventStatus<T>
where
    T: AddressSearcher,
{
    fn search_for_addresses(&self, addresses: &mut HashSet<Addr>) {
        match self {
            EventStatus::Ok(event)
            | EventStatus::NestedFailed(event)
            | EventStatus::Failed { event, .. } => event.search_for_addresses(addresses),
            _ => {},
        }
    }
}

impl AddressSearcher for SubEventStatus {
    fn search_for_addresses(&self, addresses: &mut HashSet<Addr>) {
        match self {
            SubEventStatus::Ok(event)
            | SubEventStatus::NestedFailed(event)
            | SubEventStatus::Failed { event, .. }
            | SubEventStatus::Handled { event, .. } => {
                event.search_for_addresses(addresses);
            },
        }
    }
}

impl AddressSearcher for SubEvent {
    fn search_for_addresses(&self, addresses: &mut HashSet<Addr>) {
        self.event.search_for_addresses(addresses);

        if let Some(reply) = &self.reply {
            reply.search_for_addresses(addresses);
        }
    }
}

impl AddressSearcher for CheckedContractEvent {
    fn search_for_addresses(&self, addresses: &mut HashSet<Addr>) {
        self.data.inner().search_for_addresses(addresses);
    }
}

impl AddressSearcher for JsonValue {
    fn search_for_addresses(&self, addresses: &mut HashSet<Addr>) {
        match self {
            JsonValue::String(string) => {
                if let Ok(address) = Addr::from_str(string) {
                    addresses.insert(address);
                }
            },
            JsonValue::Array(array) => {
                for item in array {
                    item.search_for_addresses(addresses);
                }
            },
            JsonValue::Object(object) => {
                for value in object.values() {
                    value.search_for_addresses(addresses);
                }
            },
            _ => {},
        }
    }
}
