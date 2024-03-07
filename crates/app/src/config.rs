use {
    super::new_update_config_event,
    crate::{AppError, AppResult, CONFIG},
    cw_std::{Addr, Config, Event, Storage},
    tracing::{info, warn},
};

pub fn do_update_config(
    store:   &mut dyn Storage,
    sender:  &Addr,
    new_cfg: &Config,
) -> AppResult<Vec<Event>> {
    match _do_update_config(store, sender, new_cfg) {
        Ok(events) => {
            info!("Config updated");
            Ok(events)
        },
        Err(err) => {
            warn!(err = err.to_string(), "Failed to update config");
            Err(err)
        },
    }
}

fn _do_update_config(
    store:   &mut dyn Storage,
    sender:  &Addr,
    new_cfg: &Config,
) -> AppResult<Vec<Event>> {
    // make sure the sender is authorized to update the config
    let cfg = CONFIG.load(store)?;
    let Some(owner) = cfg.owner else {
        return Err(AppError::OwnerNotSet);
    };
    if sender != owner {
        return Err(AppError::not_owner(sender.clone(), owner));
    }

    // save the new config
    CONFIG.save(store, new_cfg)?;

    Ok(vec![new_update_config_event(sender)])
}
