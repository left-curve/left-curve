use {
    crate::{new_set_config_event, AppError, AppResult, CONFIG},
    grug_types::{Addr, Config, Event, Storage},
    tracing::{info, warn},
};

pub fn do_set_config(
    storage: &mut dyn Storage,
    sender: &Addr,
    new_cfg: &Config,
) -> AppResult<Vec<Event>> {
    match _do_set_config(storage, sender, new_cfg) {
        Ok(events) => {
            info!("Config set");
            Ok(events)
        },
        Err(err) => {
            warn!(err = err.to_string(), "Failed to set config");
            Err(err)
        },
    }
}

fn _do_set_config(
    storage: &mut dyn Storage,
    sender: &Addr,
    new_cfg: &Config,
) -> AppResult<Vec<Event>> {
    // make sure the sender is authorized to set the config
    let cfg = CONFIG.load(storage)?;
    let Some(owner) = cfg.owner else {
        return Err(AppError::OwnerNotSet);
    };
    if sender != owner {
        return Err(AppError::not_owner(sender.clone(), owner));
    }

    // save the new config
    CONFIG.save(storage, new_cfg)?;

    Ok(vec![new_set_config_event(sender)])
}
