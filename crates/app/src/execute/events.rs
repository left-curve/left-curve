use cw_std::{Addr, Attribute, Event, Hash};

const CONTRACT_ADDRESS_KEY: &str = "_contract_address";

pub fn new_update_config_event(sender: &Addr) -> Event {
    Event::new("update_config")
        .add_attribute("sender", sender)
}

pub fn new_store_code_event(code_hash: &Hash, uploader: &Addr) -> Event {
    Event::new("store_code")
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
