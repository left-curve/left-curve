use {
    grug_types::{Addr, FlatEvent, FlatEventInfo, Inner, Json},
    std::{collections::HashSet, str::FromStr},
};

pub fn find_addresses_in_event(event: &FlatEventInfo) -> HashSet<Addr> {
    match &event.event {
        FlatEvent::Configure(evt) => HashSet::from([evt.sender]),
        FlatEvent::Transfer(evt) => {
            let mut set = HashSet::with_capacity(evt.transfers.len() + 1);
            set.insert(evt.sender);
            for sender in evt.transfers.keys() {
                set.insert(*sender);
            }
            set
        },
        FlatEvent::Upload(evt) => HashSet::from([evt.sender]),
        FlatEvent::Instantiate(evt) => {
            let mut addresses = find_addresses_in_json(&evt.instantiate_msg);
            addresses.extend([evt.contract, evt.sender]);
            if let Some(admin) = evt.admin {
                addresses.insert(admin);
            }
            addresses
        },
        FlatEvent::Execute(evt) => {
            let mut addresses = find_addresses_in_json(&evt.execute_msg);
            addresses.extend([evt.contract, evt.sender]);
            addresses
        },
        FlatEvent::Migrate(evt) => {
            let mut addresses = find_addresses_in_json(&evt.migrate_msg);
            addresses.extend([evt.sender, evt.contract]);
            addresses
        },
        FlatEvent::Reply(evt) => {
            // Probably is better to skip searching addresses in the ReplyEvent.

            // let mut addresses = match &evt.reply_on {
            //     grug_types::ReplyOn::Success(json)
            //     | grug_types::ReplyOn::Error(json)
            //     | grug_types::ReplyOn::Always(json) => find_addresses_in_json(json),
            //     grug_types::ReplyOn::Never => HashSet::with_capacity(1),
            // };
            // addresses.insert(evt.contract);
            // addresses
            [evt.contract].into()
        },
        FlatEvent::Authenticate(evt) => [evt.sender].into(),
        FlatEvent::Backrun(evt) => [evt.sender].into(),
        FlatEvent::Withhold(evt) => {
            if let Some(taxman) = evt.taxman {
                [taxman, evt.sender].into()
            } else {
                [evt.sender].into()
            }
        },
        FlatEvent::Finalize(evt) => {
            if let Some(taxman) = evt.taxman {
                [taxman, evt.sender].into()
            } else {
                [evt.sender].into()
            }
        },
        FlatEvent::Cron(evt) => [evt.contract].into(),
        FlatEvent::Guest(evt) => [evt.contract].into(),
        FlatEvent::ContractEvent(evt) => {
            let mut addresses = find_addresses_in_json(&evt.data);
            addresses.insert(evt.contract);
            addresses
        },
    }
}

fn find_addresses_in_json(json: &Json) -> HashSet<Addr> {
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
                    for (key, json) in map {
                        if let Ok(addr) = Addr::from_str(key) {
                            addresses.insert(addr);
                        } else {
                            #[cfg(feature = "tracing")]
                            tracing::warn!("key is not an address but it should be: {key}");
                        }

                        has_address = recursive(json, addresses) || has_address;

                        // If the path is None, it means that each element of the array can't contains an address.
                        // This because we assume that this array is a json rappresentation of a typed array
                        if !has_address {
                            return false;
                        }
                    }

                    has_address
                } else {
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
    recursive(json.inner(), &mut addresses);
    addresses
}

#[cfg(test)]
mod tests {
    use {super::*, grug_types::json};

    #[test]
    fn test_search_json() {
        let json = json!(
            {
                "a": Addr::mock(1),
                "b": {
                    Addr::mock(2).to_string(): Addr::mock(2).to_string(),
                    Addr::mock(3).to_string(): Addr::mock(4).to_string(),
                },
                "c": 2,
                "d": {
                    Addr::mock(5).to_string(): {"user": Addr::mock(6).to_string(), "amount": "100"},
                    Addr::mock(6).to_string(): {"user": Addr::mock(7).to_string(), "amount": "100"},
                },
                "e": [
                    Addr::mock(8).to_string(),
                    Addr::mock(9).to_string()
                ],
                "f": {},
            }
        );
        let addresses = find_addresses_in_json(&json);
        assert_eq!(addresses.len(), 9);
    }

    #[test]
    fn empty_and_null_are_ignored_and_continued() {
        let json = json!([[], [Addr::mock(2).to_string(), Addr::mock(3).to_string()]]);

        let addresses = find_addresses_in_json(&json);
        assert_eq!(addresses.len(), 2);

        let json = json!([{}, {
            "a": Addr::mock(1).to_string(),
            "b": Addr::mock(2).to_string()
        }]);

        let addresses = find_addresses_in_json(&json);
        assert_eq!(addresses.len(), 2);

        let json = json!([{
            "a": null,
            "b": null
        }, {
            "a": Addr::mock(1).to_string(),
            "b": Addr::mock(2).to_string()
        }]);

        let addresses = find_addresses_in_json(&json);
        assert_eq!(addresses.len(), 2);
    }

    #[test]
    fn non_rust_struct_json() {
        let json = json!([[1], [Addr::mock(2).to_string(), Addr::mock(3).to_string()]]);

        let addresses = find_addresses_in_json(&json);
        assert_eq!(addresses.len(), 0);
    }
}
