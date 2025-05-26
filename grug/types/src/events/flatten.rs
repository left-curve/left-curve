use crate::{
    Event, EventId, EventStatus, EvtAuthenticate, EvtBackrun, EvtConfigure, EvtCron, EvtExecute,
    EvtFinalize, EvtGuest, EvtInstantiate, EvtMigrate, EvtReply, EvtTransfer, EvtUpload,
    EvtWithhold, FlatCommitmentStatus, FlatEvent, FlatEventInfo, FlatEventStatus,
    FlatEvtAuthenticate, FlatEvtBackrun, FlatEvtCron, FlatEvtExecute, FlatEvtFinalize,
    FlatEvtGuest, FlatEvtInstantiate, FlatEvtMigrate, FlatEvtReply, FlatEvtTransfer,
    FlatEvtWithhold, MsgsAndBackrunEvents, SubEvent, SubEventStatus,
};

pub trait Flatten {
    fn flatten(
        self,
        parent_id: &EventId,
        next_id: &mut EventId,
        commitment: FlatCommitmentStatus,
        status: FlatEventStatus,
    ) -> Vec<FlatEventInfo>;
}

pub trait FlattenStatus {
    fn flatten_status(
        self,
        parent_id: &EventId,
        next_id: &mut EventId,
        commitment: FlatCommitmentStatus,
    ) -> Vec<FlatEventInfo>;
}

// -------------------------- impl Flat for Status --------------------------

impl<T> FlattenStatus for EventStatus<T>
where
    T: Flatten,
{
    fn flatten_status(
        self,
        parent_id: &EventId,
        next_id: &mut EventId,
        commitment: FlatCommitmentStatus,
    ) -> Vec<FlatEventInfo> {
        match self {
            EventStatus::Ok(event) => {
                event.flatten(parent_id, next_id, commitment, FlatEventStatus::Ok)
            },
            EventStatus::Failed { event, error } => event.flatten(
                parent_id,
                next_id,
                commitment,
                FlatEventStatus::Failed(error),
            ),
            EventStatus::NestedFailed(event) => event.flatten(
                parent_id,
                next_id,
                commitment,
                FlatEventStatus::NestedFailed,
            ),
            EventStatus::NotReached => vec![],
        }
    }
}

impl FlattenStatus for SubEventStatus {
    fn flatten_status(
        self,
        parent_id: &EventId,
        next_id: &mut EventId,
        commitment: FlatCommitmentStatus,
    ) -> Vec<FlatEventInfo> {
        let (event, commitment, status) = match self {
            SubEventStatus::Ok(event) => (event, commitment, FlatEventStatus::Ok),
            SubEventStatus::NestedFailed(event) => {
                (event, commitment, FlatEventStatus::NestedFailed)
            },
            SubEventStatus::Failed { event, error } => {
                (event, commitment, FlatEventStatus::Failed(error))
            },
            // SubEventStatus::Handled is a particular case.
            // It means that a submsg fails but the error has been handled on reply.
            // In this case, the commitment status is Failed regardless of the original commitment status.
            SubEventStatus::Handled { event, error } => (
                event,
                FlatCommitmentStatus::Failed,
                FlatEventStatus::Handled(error),
            ),
        };

        event.flatten(parent_id, next_id, commitment, status)
    }
}

impl FlattenStatus for MsgsAndBackrunEvents {
    fn flatten_status(
        self,
        parent_id: &EventId,
        next_id: &mut EventId,
        commitment: FlatCommitmentStatus,
    ) -> Vec<FlatEventInfo> {
        let mut events = vec![];

        for (msg_idx, msgs) in self.msgs.into_iter().enumerate() {
            next_id.message_index = Some(msg_idx as u32);

            let i_events = msgs.flatten_status(parent_id, next_id, commitment);

            next_id.increment_idx(&i_events);
            events.extend(i_events);
        }
        next_id.message_index = None;

        events.extend(self.backrun.flatten_status(parent_id, next_id, commitment));

        events
    }
}

// -------------------------- impl Flat for Events --------------------------

impl Flatten for Event {
    fn flatten(
        self,
        parent_id: &EventId,
        next_id: &mut EventId,
        commitment: FlatCommitmentStatus,
        status: FlatEventStatus,
    ) -> Vec<FlatEventInfo> {
        match self {
            Event::Configure(evt_configure) => {
                evt_configure.flatten(parent_id, next_id, commitment, status)
            },
            Event::Transfer(evt_transfer) => {
                evt_transfer.flatten(parent_id, next_id, commitment, status)
            },
            Event::Upload(evt_upload) => evt_upload.flatten(parent_id, next_id, commitment, status),
            Event::Instantiate(evt_instantiate) => {
                evt_instantiate.flatten(parent_id, next_id, commitment, status)
            },
            Event::Execute(evt_execute) => {
                evt_execute.flatten(parent_id, next_id, commitment, status)
            },
            Event::Migrate(evt_migrate) => {
                evt_migrate.flatten(parent_id, next_id, commitment, status)
            },
            Event::Reply(evt_reply) => evt_reply.flatten(parent_id, next_id, commitment, status),
            Event::Authenticate(evt_authenticate) => {
                evt_authenticate.flatten(parent_id, next_id, commitment, status)
            },
            Event::Backrun(evt_backrun) => {
                evt_backrun.flatten(parent_id, next_id, commitment, status)
            },
            Event::Withhold(evt_withhold) => {
                evt_withhold.flatten(parent_id, next_id, commitment, status)
            },
            Event::Finalize(evt_finalize) => {
                evt_finalize.flatten(parent_id, next_id, commitment, status)
            },
            Event::Cron(evt_cron) => evt_cron.flatten(parent_id, next_id, commitment, status),
        }
    }
}

impl Flatten for EvtConfigure {
    fn flatten(
        self,
        parent_id: &EventId,
        next_id: &mut EventId,
        commitment: FlatCommitmentStatus,
        status: FlatEventStatus,
    ) -> Vec<FlatEventInfo> {
        vec![FlatEventInfo {
            id: next_id.clone(),
            parent_id: parent_id.clone(),
            commitment_status: commitment,
            event_status: status,
            event: FlatEvent::Configure(self),
        }]
    }
}

impl Flatten for EvtTransfer {
    fn flatten(
        self,
        parent_id: &EventId,
        next_id: &mut EventId,
        commitment: FlatCommitmentStatus,
        status: FlatEventStatus,
    ) -> Vec<FlatEventInfo> {
        let mut events = vec![FlatEventInfo {
            id: next_id.clone(),
            parent_id: parent_id.clone(),
            commitment_status: commitment,
            event_status: status,
            event: FlatEvent::Transfer(FlatEvtTransfer {
                sender: self.sender,
                transfers: self.transfers,
            }),
        }];

        next_id.event_index += 1;

        let bank_guest = self
            .bank_guest
            .flatten_status(parent_id, next_id, commitment);

        next_id.increment_idx(&bank_guest);

        events.extend(bank_guest);

        for event in self.receive_guests.into_values() {
            let receive_guest = event.flatten_status(parent_id, next_id, commitment);
            next_id.increment_idx(&receive_guest);
            events.extend(receive_guest);
        }

        events
    }
}

impl Flatten for EvtUpload {
    fn flatten(
        self,
        parent_id: &EventId,
        next_id: &mut EventId,
        commitment: FlatCommitmentStatus,
        status: FlatEventStatus,
    ) -> Vec<FlatEventInfo> {
        vec![FlatEventInfo {
            id: parent_id.clone_with_event_index(next_id.event_index),
            parent_id: parent_id.clone(),
            commitment_status: commitment,
            event_status: status,
            event: FlatEvent::Upload(self),
        }]
    }
}

impl Flatten for EvtInstantiate {
    fn flatten(
        self,
        parent_id: &EventId,
        next_id: &mut EventId,
        commitment: FlatCommitmentStatus,
        status: FlatEventStatus,
    ) -> Vec<FlatEventInfo> {
        let mut events = vec![FlatEventInfo {
            id: next_id.clone(),
            parent_id: parent_id.clone(),
            commitment_status: commitment,
            event_status: status,
            event: FlatEvent::Instantiate(FlatEvtInstantiate {
                sender: self.sender,
                contract: self.contract,
                code_hash: self.code_hash,
                label: self.label,
                admin: self.admin,
                instantiate_msg: self.instantiate_msg,
            }),
        }];

        let parent_id = next_id.clone();
        next_id.event_index += 1;

        let transfer = self
            .transfer_event
            .flatten_status(&parent_id, next_id, commitment);

        next_id.increment_idx(&transfer);

        events.extend(transfer);

        let guest = self
            .guest_event
            .flatten_status(&parent_id, next_id, commitment);

        events.extend(guest);
        events
    }
}

impl Flatten for EvtExecute {
    fn flatten(
        self,
        parent_id: &EventId,
        next_id: &mut EventId,
        commitment: FlatCommitmentStatus,
        status: FlatEventStatus,
    ) -> Vec<FlatEventInfo> {
        let mut events = vec![FlatEventInfo {
            id: next_id.clone(),
            parent_id: parent_id.clone(),
            commitment_status: commitment,
            event_status: status,
            event: FlatEvent::Execute(FlatEvtExecute {
                sender: self.sender,
                contract: self.contract,
                funds: self.funds,
                execute_msg: self.execute_msg,
            }),
        }];

        let parent_id = next_id.clone();
        next_id.event_index += 1;

        let transfer = self
            .transfer_event
            .flatten_status(&parent_id, next_id, commitment);

        next_id.increment_idx(&transfer);

        events.extend(transfer);

        let guest = self
            .guest_event
            .flatten_status(&parent_id, next_id, commitment);

        events.extend(guest);
        events
    }
}

impl Flatten for EvtMigrate {
    fn flatten(
        self,
        parent_id: &EventId,
        next_id: &mut EventId,
        commitment: FlatCommitmentStatus,
        status: FlatEventStatus,
    ) -> Vec<FlatEventInfo> {
        let mut events = vec![FlatEventInfo {
            id: next_id.clone(),
            parent_id: parent_id.clone(),
            commitment_status: commitment,
            event_status: status,
            event: FlatEvent::Migrate(FlatEvtMigrate {
                sender: self.sender,
                contract: self.contract,
                migrate_msg: self.migrate_msg,
                old_code_hash: self.old_code_hash,
                new_code_hash: self.new_code_hash,
            }),
        }];

        let parent_id = next_id.clone();
        next_id.event_index += 1;

        let guest = self
            .guest_event
            .flatten_status(&parent_id, next_id, commitment);

        events.extend(guest);
        events
    }
}

impl Flatten for EvtBackrun {
    fn flatten(
        self,
        parent_id: &EventId,
        next_id: &mut EventId,
        commitment: FlatCommitmentStatus,
        status: FlatEventStatus,
    ) -> Vec<FlatEventInfo> {
        let mut events = vec![FlatEventInfo {
            id: next_id.clone(),
            parent_id: parent_id.clone(),
            commitment_status: commitment,
            event_status: status,
            event: FlatEvent::Backrun(FlatEvtBackrun {
                sender: self.sender,
            }),
        }];

        let parent_id = next_id.clone();
        next_id.event_index += 1;

        let guest = self
            .guest_event
            .flatten_status(&parent_id, next_id, commitment);

        events.extend(guest);
        events
    }
}

impl Flatten for EvtWithhold {
    fn flatten(
        self,
        parent_id: &EventId,
        next_id: &mut EventId,
        commitment: FlatCommitmentStatus,
        status: FlatEventStatus,
    ) -> Vec<FlatEventInfo> {
        let mut events = vec![FlatEventInfo {
            id: next_id.clone(),
            parent_id: parent_id.clone(),
            commitment_status: commitment,
            event_status: status,
            event: FlatEvent::Withhold(FlatEvtWithhold {
                sender: self.sender,
                gas_limit: self.gas_limit,
                taxman: self.taxman,
            }),
        }];

        let parent_id = next_id.clone();
        next_id.event_index += 1;

        let guest = self
            .guest_event
            .flatten_status(&parent_id, next_id, commitment);

        events.extend(guest);
        events
    }
}

impl Flatten for EvtFinalize {
    fn flatten(
        self,
        parent_id: &EventId,
        next_id: &mut EventId,
        commitment: FlatCommitmentStatus,
        status: FlatEventStatus,
    ) -> Vec<FlatEventInfo> {
        let mut events = vec![FlatEventInfo {
            id: next_id.clone(),
            parent_id: parent_id.clone(),
            commitment_status: commitment,
            event_status: status,
            event: FlatEvent::Finalize(FlatEvtFinalize {
                sender: self.sender,
                gas_limit: self.gas_limit,
                taxman: self.taxman,
                gas_used: self.gas_used,
            }),
        }];

        let parent_id = next_id.clone();
        next_id.event_index += 1;

        let guest = self
            .guest_event
            .flatten_status(&parent_id, next_id, commitment);

        events.extend(guest);
        events
    }
}

impl Flatten for EvtCron {
    fn flatten(
        self,
        parent_id: &EventId,
        next_id: &mut EventId,
        commitment: FlatCommitmentStatus,
        status: FlatEventStatus,
    ) -> Vec<FlatEventInfo> {
        let mut events = vec![FlatEventInfo {
            id: next_id.clone(),
            parent_id: parent_id.clone(),
            commitment_status: commitment,
            event_status: status,
            event: FlatEvent::Cron(FlatEvtCron {
                contract: self.contract,
                time: self.time,
                next: self.next,
            }),
        }];

        let parent_id = next_id.clone();
        next_id.event_index += 1;

        let guest = self
            .guest_event
            .flatten_status(&parent_id, next_id, commitment);

        events.extend(guest);
        events
    }
}

impl Flatten for EvtAuthenticate {
    fn flatten(
        self,
        parent_id: &EventId,
        next_id: &mut EventId,
        commitment: FlatCommitmentStatus,
        status: FlatEventStatus,
    ) -> Vec<FlatEventInfo> {
        let mut events = vec![FlatEventInfo {
            id: next_id.clone(),
            parent_id: parent_id.clone(),
            commitment_status: commitment,
            event_status: status,
            event: FlatEvent::Authenticate(FlatEvtAuthenticate {
                sender: self.sender,
                backrun: self.backrun,
            }),
        }];

        let parent_id = next_id.clone();
        next_id.event_index += 1;

        let guest = self
            .guest_event
            .flatten_status(&parent_id, next_id, commitment);

        events.extend(guest);
        events
    }
}

impl Flatten for EvtGuest {
    fn flatten(
        self,
        parent_id: &EventId,
        next_id: &mut EventId,
        commitment: FlatCommitmentStatus,
        status: FlatEventStatus,
    ) -> Vec<FlatEventInfo> {
        let current_id = parent_id.clone_with_event_index(next_id.event_index);

        let mut events = vec![FlatEventInfo {
            id: next_id.clone(),
            parent_id: parent_id.clone(),
            commitment_status: commitment,
            event_status: status.clone(),
            event: FlatEvent::Guest(FlatEvtGuest {
                contract: self.contract,
                method: self.method,
            }),
        }];

        next_id.event_index += 1;

        for contract_event in self.contract_events {
            events.push(FlatEventInfo {
                id: next_id.clone(),
                parent_id: parent_id.clone(),
                commitment_status: commitment,
                event_status: status.clone(),
                event: FlatEvent::ContractEvent(contract_event),
            });

            next_id.event_index += 1;
        }

        for sub_event in self.sub_events {
            let sub_events = sub_event.flatten_status(&current_id, next_id, commitment);
            next_id.increment_idx(&sub_events);
            events.extend(sub_events);
        }

        events
    }
}

impl Flatten for SubEvent {
    fn flatten(
        self,
        parent_id: &EventId,
        next_id: &mut EventId,
        commitment: FlatCommitmentStatus,
        _status: FlatEventStatus,
    ) -> Vec<FlatEventInfo> {
        let mut events = vec![];

        let sub_event = self
            .event
            .flatten_status(parent_id, next_id, commitment);

        let reply_events = if let Some(reply) = self.reply {
            next_id.increment_idx(&sub_event);
            reply.flatten_status(parent_id, next_id, commitment)
        } else {
            vec![]
        };

        events.extend(sub_event);
        events.extend(reply_events);

        events
    }
}

impl Flatten for EvtReply {
    fn flatten(
        self,
        parent_id: &EventId,
        next_id: &mut EventId,
        commitment: FlatCommitmentStatus,
        status: FlatEventStatus,
    ) -> Vec<FlatEventInfo> {
        let mut events = vec![FlatEventInfo {
            id: next_id.clone(),
            parent_id: parent_id.clone(),
            commitment_status: commitment,
            event_status: status,
            event: FlatEvent::Reply(FlatEvtReply {
                contract: self.contract,
                reply_on: self.reply_on,
            }),
        }];

        let parent_id = next_id.clone();
        next_id.event_index += 1;

        let guest = self
            .guest_event
            .flatten_status(&parent_id, next_id, commitment);

        events.extend(guest);
        events
    }
}
