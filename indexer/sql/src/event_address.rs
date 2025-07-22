use {
    anyhow::bail,
    grug_types::{Addr, FlatEvent, FlatEventInfo, Inner, Json},
    serde_json::Value,
    std::{
        collections::{HashMap, HashSet},
        str::FromStr,
    },
};

#[derive(Default)]
pub struct AddressFinder {}

impl AddressFinder {
    pub fn find_addresses(&mut self, event: &FlatEventInfo) -> HashSet<Addr> {
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
                todo!()
            },
            FlatEvent::Execute(flat_evt_execute) => todo!(),
            FlatEvent::Migrate(flat_evt_migrate) => todo!(),
            FlatEvent::Reply(flat_evt_reply) => todo!(),
            FlatEvent::Authenticate(flat_evt_authenticate) => todo!(),
            FlatEvent::Backrun(flat_evt_backrun) => todo!(),
            FlatEvent::Withhold(flat_evt_withhold) => todo!(),
            FlatEvent::Finalize(flat_evt_finalize) => todo!(),
            FlatEvent::Cron(flat_evt_cron) => todo!(),
            FlatEvent::Guest(flat_evt_guest) => todo!(),
            FlatEvent::ContractEvent(checked_contract_event) => todo!(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
enum PathType {
    MapAndAllKeysAreAddresses(Box<PathType>),
    Map(HashMap<String, Box<PathType>>),
    Array(Box<PathType>),
    Value,
}

fn search_json(json: &Json, path: Option<PathType>) -> (HashSet<Addr>, Option<PathType>) {
    fn recursive(json: &serde_json::Value, addresses: &mut HashSet<Addr>) -> Option<PathType> {
        match json {
            serde_json::Value::String(val) => {
                if let Ok(addr) = Addr::from_str(val) {
                    addresses.insert(addr);
                    Some(PathType::Value)
                } else {
                    None
                }
            },
            serde_json::Value::Array(values) => {
                let mut path = None;
                for value in values {
                    path = recursive(value, addresses);
                }
                if let Some(path) = path {
                    Some(PathType::Array(Box::new(path)))
                } else {
                    None
                }
            },
            serde_json::Value::Object(map) => {
                let mut is_key_addr = true;

                if let Some(k) = map.keys().next() {
                    if Addr::from_str(k).is_err() {
                        is_key_addr = false;
                    }
                } else {
                    return None;
                }

                if is_key_addr {
                    let mut last = None;

                    for (key, json) in map {
                        if let Ok(addr) = Addr::from_str(key) {
                            addresses.insert(addr);
                        }

                        last = recursive(json, addresses);
                    }

                    if let Some(last) = last {
                        Some(PathType::MapAndAllKeysAreAddresses(Box::new(last)))
                    } else {
                        None
                    }
                } else {
                    let mut path_map = HashMap::default();
                    for (key, json) in map {
                        if let Some(path) = recursive(json, addresses) {
                            path_map.insert(key.clone(), Box::new(path));
                        }
                    }

                    if path_map.is_empty() {
                        None
                    } else {
                        Some(PathType::Map(path_map))
                    }
                }
            },
            _ => None,
        }
    }

    fn recursive_with_path(
        json: &serde_json::Value,
        path: &PathType,
        addresses: &mut HashSet<Addr>,
    ) -> anyhow::Result<()> {
        match (json, path) {
            (Value::String(val), PathType::Value) => match Addr::from_str(val) {
                Ok(addr) => {
                    addresses.insert(addr);
                    Ok(())
                },
                Err(e) => Err(e.into()),
            },
            (Value::Array(values), PathType::Array(path)) => {
                for value in values {
                    recursive_with_path(value, path, addresses)?;
                }
                Ok(())
            },
            (Value::Object(map), PathType::Map(path)) => {
                for (key, path) in path {
                    let Some(json) = map.get(key) else {
                        return Err(anyhow::anyhow!("key not found"));
                    };

                    recursive_with_path(json, path, addresses)?;
                }
                Ok(())
            },
            (Value::Object(map), PathType::MapAndAllKeysAreAddresses(path)) => {
                for (key, json) in map {
                    match Addr::from_str(&key) {
                        Ok(addr) => {
                            addresses.insert(addr);
                        },
                        Err(e) => return Err(e.into()),
                    }
                    recursive_with_path(json, path, addresses)?;
                }
                Ok(())
            },
            _ => bail!("path not correct"),
        }
    }

    let mut addresses = HashSet::default();

    if let Some(path) = path {
        if recursive_with_path(json.inner(), &path, &mut addresses).is_err() {
            let mut addresses = HashSet::default();

            let path = recursive(&json.inner(), &mut addresses);
            return (addresses, path);
        }
        (addresses, Some(path))
    } else {
        let path = recursive(json.inner(), &mut addresses);
        (addresses, path)
    }
}

#[cfg(test)]
mod tests {
    use grug_types::json;

    use super::*;

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
                }
            }
        );
        let (addresses, path) = search_json(&json, None);
        assert_eq!(addresses.len(), 7);

        println!("{:?}", path);

        let (addresses_2, path_2) = search_json(&json, path.clone());

        assert_eq!(addresses, addresses_2);
        assert_eq!(path, path_2);
    }
}
