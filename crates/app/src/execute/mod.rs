mod auth;
mod config;
mod events;
#[allow(clippy::module_inception)]
mod execute;
mod instantiate;
mod migrate;
mod store;
mod submessage;
mod transfer;

pub use auth::authenticate_tx;

use {
    crate::AppResult,
    config::update_config,
    cw_db::{Flush, Storage},
    cw_std::{Addr, BlockInfo, Event, Message},
    events::{
        new_before_tx_event, new_execute_event, new_instantiate_event, new_migrate_event,
        new_receive_event, new_reply_event, new_store_code_event, new_transfer_event,
        new_update_config_event,
    },
    execute::execute,
    instantiate::instantiate,
    migrate::migrate,
    store::store_code,
    submessage::handle_submessages,
    transfer::transfer,
};

pub fn process_msg<S: Storage + Flush + Clone + 'static>(
    mut store: S,
    block: &BlockInfo,
    sender: &Addr,
    msg: Message,
) -> AppResult<Vec<Event>> {
    match msg {
        Message::UpdateConfig {
            new_cfg,
        } => update_config(&mut store, sender, &new_cfg),
        Message::Transfer {
            to,
            coins,
        } => transfer(store, block, sender.clone(), to, coins),
        Message::StoreCode {
            wasm_byte_code,
        } => store_code(&mut store, sender, &wasm_byte_code),
        Message::Instantiate {
            code_hash,
            msg,
            salt,
            funds,
            admin,
        } => instantiate(store, block, sender, code_hash, msg, salt, funds, admin),
        Message::Execute {
            contract,
            msg,
            funds,
        } => execute(store, block, &contract, sender, msg, funds),
        Message::Migrate {
            contract,
            new_code_hash,
            msg,
        } => migrate(store, block, &contract, sender, new_code_hash, msg),
    }
}
