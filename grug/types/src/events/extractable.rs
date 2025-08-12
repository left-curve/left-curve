use {
    crate::{
        Addr, CheckedContractEvent, EvtConfigure, EvtUpload, FlatEvent, FlatEvtExecute,
        FlatEvtInstantiate, FlatEvtMigrate, FlatEvtTransfer, Inner, Json,
    },
    std::{collections::HashSet, str::FromStr},
};

pub trait Extractable {
    fn extract_addresses(&self, addresses: &mut HashSet<Addr>);
}

impl Extractable for FlatEvent {
    fn extract_addresses(&self, addresses: &mut HashSet<Addr>) {
        match self {
            FlatEvent::Configure(evt) => evt.extract_addresses(addresses),
            FlatEvent::Transfer(evt) => evt.extract_addresses(addresses),
            FlatEvent::Upload(evt) => evt.extract_addresses(addresses),
            FlatEvent::Instantiate(evt) => evt.extract_addresses(addresses),
            FlatEvent::Execute(evt) => evt.extract_addresses(addresses),
            FlatEvent::Migrate(evt) => evt.extract_addresses(addresses),
            FlatEvent::ContractEvent(evt) => evt.extract_addresses(addresses),
            _ => {
                // The other flat event types don't contain addresses that we
                // care to index. Do nothing.
            },
        }
    }
}

impl Extractable for FlatEvtTransfer {
    fn extract_addresses(&self, addresses: &mut HashSet<Addr>) {
        addresses.insert(self.sender);
        addresses.extend(self.transfers.keys());
    }
}

impl Extractable for FlatEvtInstantiate {
    fn extract_addresses(&self, addresses: &mut HashSet<Addr>) {
        addresses.extend([self.contract, self.sender]);
        if let Some(admin) = self.admin {
            addresses.insert(admin);
        }
    }
}

impl Extractable for FlatEvtExecute {
    fn extract_addresses(&self, addresses: &mut HashSet<Addr>) {
        addresses.extend([self.contract, self.sender]);
    }
}

impl Extractable for FlatEvtMigrate {
    fn extract_addresses(&self, addresses: &mut HashSet<Addr>) {
        addresses.extend([self.sender, self.contract]);
    }
}

impl Extractable for CheckedContractEvent {
    fn extract_addresses(&self, addresses: &mut HashSet<Addr>) {
        addresses.insert(self.contract);
    }
}

impl Extractable for EvtConfigure {
    fn extract_addresses(&self, addresses: &mut HashSet<Addr>) {
        addresses.insert(self.sender);
    }
}

impl Extractable for EvtUpload {
    fn extract_addresses(&self, addresses: &mut HashSet<Addr>) {
        addresses.insert(self.sender);
    }
}

impl Extractable for Json {
    fn extract_addresses(&self, addresses: &mut HashSet<Addr>) {
        self.inner().extract_addresses(addresses);
    }
}

impl Extractable for serde_json::Value {
    fn extract_addresses(&self, addresses: &mut HashSet<Addr>) {
        match self {
            serde_json::Value::String(s) => s.extract_addresses(addresses),
            serde_json::Value::Array(a) => a.extract_addresses(addresses),
            serde_json::Value::Object(o) => o.extract_addresses(addresses),
            _ => { /* Does not contain addresses; do nothing. */ },
        }
    }
}

impl Extractable for String {
    fn extract_addresses(&self, addresses: &mut HashSet<Addr>) {
        if let Ok(addr) = Addr::from_str(self) {
            addresses.insert(addr);
        }
    }
}

impl Extractable for Vec<serde_json::Value> {
    fn extract_addresses(&self, addresses: &mut HashSet<Addr>) {
        for value in self {
            value.extract_addresses(addresses);
        }
    }
}

impl Extractable for serde_json::Map<String, serde_json::Value> {
    fn extract_addresses(&self, addresses: &mut HashSet<Addr>) {
        for (key, value) in self {
            key.extract_addresses(addresses);
            value.extract_addresses(addresses);
        }
    }
}

// ----------------------------------- tests -----------------------------------

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

        let mut addresses = HashSet::new();
        json.extract_addresses(&mut addresses);
        assert_eq!(addresses.len(), 13);
    }

    #[test]
    fn key_are_addresses_but_not_values() {
        let json = json!({
            Addr::mock(1).to_string(): 1,
            Addr::mock(2).to_string(): 2,
        });

        let mut addresses = HashSet::new();
        json.extract_addresses(&mut addresses);
        assert_eq!(addresses.len(), 2);
    }

    #[test]
    fn empty_and_null_are_ignored_and_continued() {
        let json = json!([[], [Addr::mock(2).to_string(), Addr::mock(3).to_string()]]);

        let mut addresses = HashSet::new();
        json.extract_addresses(&mut addresses);
        assert_eq!(addresses.len(), 2);

        let json = json!([{}, {
            "a": Addr::mock(1).to_string(),
            "b": Addr::mock(2).to_string()
        }]);

        let mut addresses = HashSet::new();
        json.extract_addresses(&mut addresses);
        assert_eq!(addresses.len(), 2);

        let json = json!([{
            "a": null,
            "b": null
        }, {
            "a": Addr::mock(1).to_string(),
            "b": Addr::mock(2).to_string()
        }]);

        let mut addresses = HashSet::new();
        json.extract_addresses(&mut addresses);
        assert_eq!(addresses.len(), 2);
    }
}
