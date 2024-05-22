use grug_types::{Addr, Attribute, Event, Hash};

// Event attribute keys emitted by the state machine are prefixed by an
// underscore. Contracts are not allowed to emit event attributes whose keys are
// similarly prefixed. This prevents malicious contracts from emitting an
// attribute that impersonates state machine attributes in order to fool indexers.
const CONTRACT_ADDRESS_KEY: &str = "_contract_address";

// Below: IBC event attribute keys.
// For IBC events, we keep them consistent with ibc-go, which may make relayer
// itegration easier.
// E.g. instead of `_contract_address` we use `client_id`; instead of `code_hash`
// we use `client_type`.
// This also means we can't prefix these keys with an underscore like we do with
// `_contract_address`, but this is fine because IBC clients are permissioned
// (only codes approved by governance and added to the chain `Config` can be
// used to create IBC clients).

/// Attribute key representing the identifier of an IBC client.
///
/// In ibc-go, this is a string such as `07-tendermint-1`. In our case, this is
/// a contract address.
const CLIENT_ID_KEY: &str = "client_id";

/// Attribute key representing the type of an IBC client.
///
/// In ibc-go, this is a string such as `07-tendermint`. In out case, this is
/// the client contract's Wasm code hash.
const CLIENT_TYPE_KEY: &str = "client_type";

pub fn new_set_config_event(sender: &Addr) -> Event {
    Event::new("set_config")
        .add_attribute("sender", sender)
}

pub fn new_upload_event(code_hash: &Hash, uploader: &Addr) -> Event {
    Event::new("upload")
        .add_attribute("hash", code_hash)
        .add_attribute("uploader", uploader)
}

pub fn new_before_block_event(sender: &Addr, attrs: Vec<Attribute>) -> Event {
    Event::new("before_block")
        .add_attribute(CONTRACT_ADDRESS_KEY, sender)
        .add_attributes(attrs)
}

pub fn new_after_block_event(sender: &Addr, attrs: Vec<Attribute>) -> Event {
    Event::new("after_block")
        .add_attribute(CONTRACT_ADDRESS_KEY, sender)
        .add_attributes(attrs)
}

pub fn new_before_tx_event(sender: &Addr, attrs: Vec<Attribute>) -> Event {
    Event::new("before_tx")
        .add_attribute(CONTRACT_ADDRESS_KEY, sender)
        .add_attributes(attrs)
}

pub fn new_after_tx_event(sender: &Addr, attrs: Vec<Attribute>) -> Event {
    Event::new("after_tx")
        .add_attribute(CONTRACT_ADDRESS_KEY, sender)
        .add_attributes(attrs)
}

pub fn new_transfer_event(bank: &Addr, attrs: Vec<Attribute>) -> Event {
    Event::new("transfer")
        .add_attribute(CONTRACT_ADDRESS_KEY, bank)
        .add_attributes(attrs)
}

pub fn new_receive_event(receiver: &Addr, attrs: Vec<Attribute>) -> Event {
    Event::new("receive")
        .add_attribute(CONTRACT_ADDRESS_KEY, receiver)
        .add_attributes(attrs)
}

pub fn new_instantiate_event(contract: &Addr, code_hash: &Hash, attrs: Vec<Attribute>) -> Event {
    Event::new("instantiate")
        .add_attribute(CONTRACT_ADDRESS_KEY, contract)
        .add_attribute("code_hash", code_hash)
        .add_attributes(attrs)
}

pub fn new_execute_event(contract: &Addr, attrs: Vec<Attribute>) -> Event {
    Event::new("execute")
        .add_attribute(CONTRACT_ADDRESS_KEY, contract)
        .add_attributes(attrs)
}

pub fn new_migrate_event(
    contract:      &Addr,
    old_code_hash: &Hash,
    new_code_hash: &Hash,
    attrs: Vec<Attribute>,
) -> Event {
    Event::new("migrate")
        .add_attribute(CONTRACT_ADDRESS_KEY, contract)
        .add_attribute("old_code_hash", old_code_hash)
        .add_attribute("new_code_hash", new_code_hash)
        .add_attributes(attrs)
}

pub fn new_reply_event(contract: &Addr, attrs: Vec<Attribute>) -> Event {
    Event::new("reply")
        .add_attribute(CONTRACT_ADDRESS_KEY, contract)
        .add_attributes(attrs)
}

pub fn new_create_client_event(client: &Addr, code_hash: &Hash, attrs: Vec<Attribute>) -> Event {
    Event::new("create_client")
        .add_attribute(CLIENT_ID_KEY, client)
        .add_attribute(CLIENT_TYPE_KEY, code_hash)
        .add_attributes(attrs)
}

pub fn new_update_client_event(client: &Addr, code_hash: &Hash, attrs: Vec<Attribute>) -> Event {
    Event::new("update_client")
        .add_attribute(CLIENT_ID_KEY, client)
        .add_attribute(CLIENT_TYPE_KEY, code_hash)
        .add_attributes(attrs)
}

pub fn new_client_misbehavior_event(client: &Addr, code_hash: &Hash, attrs: Vec<Attribute>) -> Event {
    Event::new("client_misbehavior")
        .add_attribute(CLIENT_ID_KEY, client)
        .add_attribute(CLIENT_TYPE_KEY, code_hash)
        .add_attributes(attrs)
}
