use {
    crate::{
        Addr, CheckedContractEvent, EvtConfigure, EvtUpload, FlatEvent, FlatEvtAuthenticate,
        FlatEvtBackrun, FlatEvtCron, FlatEvtExecute, FlatEvtFinalize, FlatEvtGuest,
        FlatEvtInstantiate, FlatEvtMigrate, FlatEvtReply, FlatEvtTransfer, FlatEvtWithhold, Inner,
        Json,
    },
    std::{collections::HashSet, str::FromStr},
};

pub trait Extractable {
    fn extract_addresses(&self) -> HashSet<Addr> {
        HashSet::new()
    }
}

impl Extractable for FlatEvent {
    fn extract_addresses(&self) -> HashSet<Addr> {
        match self {
            FlatEvent::Configure(evt) => evt.extract_addresses(),
            FlatEvent::Transfer(evt) => evt.extract_addresses(),
            FlatEvent::Upload(evt) => evt.extract_addresses(),
            FlatEvent::Instantiate(evt) => evt.extract_addresses(),
            FlatEvent::Execute(evt) => evt.extract_addresses(),
            FlatEvent::Migrate(evt) => evt.extract_addresses(),
            FlatEvent::Reply(evt) => evt.extract_addresses(),
            FlatEvent::Authenticate(evt) => evt.extract_addresses(),
            FlatEvent::Backrun(evt) => evt.extract_addresses(),
            FlatEvent::Withhold(evt) => evt.extract_addresses(),
            FlatEvent::Finalize(evt) => evt.extract_addresses(),
            FlatEvent::Cron(evt) => evt.extract_addresses(),
            FlatEvent::Guest(evt) => evt.extract_addresses(),
            FlatEvent::ContractEvent(evt) => evt.extract_addresses(),
        }
    }
}

impl Extractable for FlatEvtAuthenticate {}

impl Extractable for FlatEvtFinalize {}

impl Extractable for FlatEvtBackrun {}

impl Extractable for FlatEvtCron {}

impl Extractable for FlatEvtWithhold {}

impl Extractable for FlatEvtReply {}

impl Extractable for FlatEvtGuest {}

impl Extractable for FlatEvtTransfer {
    fn extract_addresses(&self) -> HashSet<Addr> {
        let mut set = HashSet::with_capacity(self.transfers.len() + 1);
        set.insert(self.sender);
        for sender in self.transfers.keys() {
            set.insert(*sender);
        }
        set
    }
}

impl Extractable for FlatEvtInstantiate {
    fn extract_addresses(&self) -> HashSet<Addr> {
        let mut addresses = self.instantiate_msg.extract_addresses();
        addresses.extend([self.contract, self.sender]);
        if let Some(admin) = self.admin {
            addresses.insert(admin);
        }
        addresses
    }
}

impl Extractable for FlatEvtExecute {
    fn extract_addresses(&self) -> HashSet<Addr> {
        let mut addresses = self.execute_msg.extract_addresses();
        addresses.extend([self.contract, self.sender]);
        addresses
    }
}

impl Extractable for FlatEvtMigrate {
    fn extract_addresses(&self) -> HashSet<Addr> {
        let mut addresses = self.migrate_msg.extract_addresses();
        addresses.extend([self.sender, self.contract]);
        addresses
    }
}

impl Extractable for CheckedContractEvent {
    fn extract_addresses(&self) -> HashSet<Addr> {
        let mut addresses = self.data.extract_addresses();
        addresses.insert(self.contract);
        addresses
    }
}

impl Extractable for EvtConfigure {
    fn extract_addresses(&self) -> HashSet<Addr> {
        [self.sender].into()
    }
}

impl Extractable for EvtUpload {
    fn extract_addresses(&self) -> HashSet<Addr> {
        [self.sender].into()
    }
}

impl Extractable for Json {
    fn extract_addresses(&self) -> HashSet<Addr> {
        fn recursive(json: &serde_json::Value, addresses: &mut HashSet<Addr>) -> bool {
            match json {
                serde_json::Value::String(val) => {
                    if let Ok(addr) = Addr::from_str(val) {
                        addresses.insert(addr);
                        true
                    } else {
                        false
                    }
                },
                // Similar concept for empty Array and Object. This could be an address
                serde_json::Value::Null => true,
                serde_json::Value::Array(values) => {
                    let mut has_address = false;

                    // if the array is empty, it could be have an address inside, but is not possible to know
                    if values.is_empty() {
                        return true;
                    }

                    for json in values {
                        has_address = recursive(json, addresses) || has_address;

                        // If has_address is false, it means that each element of the Array can't contains an address.
                        // This because we assume that this Array is a json rappresentation of a Rust typed Array
                        if !has_address {
                            return false;
                        }
                    }

                    has_address
                },
                serde_json::Value::Object(map) => {
                    let mut is_key_addr = true;
                    let mut has_address = false;

                    if let Some(k) = map.keys().next() {
                        if Addr::from_str(k).is_err() {
                            is_key_addr = false
                        }
                    } else {
                        // If the object is empty, it could be have an address inside, but is not possible to know
                        return true;
                    }

                    // If the key is an address, it means we have found a Map<Addr, T>.
                    // We expect that all keys are addresses, and T has the same json struct
                    if is_key_addr {
                        // Set has_address to true to enter in the if the first time.
                        // Then if has_address becomes false, we can skip the recursive call on values
                        // because we expect that T is always the same json struct
                        // but we need to keep parsing keys.
                        has_address = true;
                        for (key, json) in map {
                            if let Ok(addr) = Addr::from_str(key) {
                                addresses.insert(addr);
                            }

                            if has_address {
                                has_address = recursive(json, addresses) || has_address;
                            }
                        }

                        has_address
                    } else {
                        // In this case we could be in Map<K, V> or in a Rust struct.
                        // We need to parse all values to find addresses.
                        for json in map.values() {
                            has_address = recursive(json, addresses) || has_address;
                        }

                        has_address
                    }
                },
                _ => false,
            }
        }
        let mut addresses = HashSet::default();
        recursive(self.inner(), &mut addresses);
        addresses
    }
}

#[cfg(test)]
mod tests {
    use {super::*, crate::json};

    #[test]
    fn search_addresses() {
        let json = json!(
            {
                "a": Addr::mock(1),
                "b": {
                    Addr::mock(2).to_string(): Addr::mock(4).to_string(),
                    Addr::mock(3).to_string(): Addr::mock(5).to_string(),
                },
                "c": 2,
                "d": {
                    Addr::mock(6).to_string(): {
                        "user": Addr::mock(8).to_string(),
                        "amount": "100",
                    },
                    Addr::mock(7).to_string(): {
                        "user": Addr::mock(9).to_string(),
                        "amount": "100",
                    },
                },
                "e": [
                    Addr::mock(10).to_string(),
                    Addr::mock(11).to_string()
                ],
                "f": {},
                "g" :{
                    "sender": Addr::mock(12).to_string(),
                    "receiver": Addr::mock(13).to_string(),
                }
            }
        );
        let addresses = json.extract_addresses();
        assert_eq!(addresses.len(), 13);
    }

    #[test]
    fn key_are_addresses_but_not_values() {
        let json = json!({
            Addr::mock(1).to_string(): 1,
            Addr::mock(2).to_string(): 2,
        });
        let addresses = json.extract_addresses();
        assert_eq!(addresses.len(), 2);
    }

    #[test]
    fn empty_and_null_are_ignored_and_continued() {
        let json = json!([[], [Addr::mock(2).to_string(), Addr::mock(3).to_string()]]);

        let addresses = json.extract_addresses();
        assert_eq!(addresses.len(), 2);

        let json = json!([{}, {
            "a": Addr::mock(1).to_string(),
            "b": Addr::mock(2).to_string()
        }]);

        let addresses = json.extract_addresses();
        assert_eq!(addresses.len(), 2);

        let json = json!([{
            "a": null,
            "b": null
        }, {
            "a": Addr::mock(1).to_string(),
            "b": Addr::mock(2).to_string()
        }]);

        let addresses = json.extract_addresses();
        assert_eq!(addresses.len(), 2);
    }

    #[test]
    fn non_rust_struct_json() {
        let json = json!([[1], [Addr::mock(2).to_string(), Addr::mock(3).to_string()]]);

        let addresses = json.extract_addresses();
        assert_eq!(addresses.len(), 0);
    }
}
