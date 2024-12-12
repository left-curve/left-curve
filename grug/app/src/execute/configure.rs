use {
    crate::{
        schedule_cronjob, AppError, AppResult, EventResult, APP_CONFIG, CONFIG, NEXT_CRONJOBS,
    },
    grug_types::{Addr, BlockInfo, EvtConfigure, MsgConfigure, Storage},
};

pub fn do_configure(
    storage: &mut dyn Storage,
    block: BlockInfo,
    sender: Addr,
    msg: MsgConfigure,
) -> EventResult<EvtConfigure> {
    let evt = EvtConfigure { sender };

    match _do_configure(storage, block, sender, msg) {
        Ok(_) => {
            #[cfg(feature = "tracing")]
            tracing::info!("Config updated");

            EventResult::Ok(evt)
        },
        Err(err) => {
            #[cfg(feature = "tracing")]
            tracing::warn!(err = err.to_string(), "Failed to updated config");

            EventResult::err(evt, err)
        },
    }
}

fn _do_configure(
    storage: &mut dyn Storage,
    block: BlockInfo,
    sender: Addr,
    msg: MsgConfigure,
) -> AppResult<()> {
    let cfg = CONFIG.load(storage)?;

    // Make sure the sender is authorized to set the config.
    if sender != cfg.owner {
        return Err(AppError::NotOwner {
            sender,
            owner: cfg.owner,
        });
    }

    if let Some(new_cfg) = msg.new_cfg {
        // If the list of cronjobs has been changed, we have to delete the
        // existing scheduled ones and reschedule.
        if new_cfg.cronjobs != cfg.cronjobs {
            NEXT_CRONJOBS.clear(storage, None, None);

            for (contract, interval) in &new_cfg.cronjobs {
                schedule_cronjob(storage, *contract, block.timestamp + *interval)?;
            }
        }

        CONFIG.save(storage, &new_cfg)?;
    }

    if let Some(new_app_cfg) = msg.new_app_cfg {
        APP_CONFIG.save(storage, &new_app_cfg)?;
    }

    Ok(())
}
