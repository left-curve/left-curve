#[cfg(feature = "tracing")]
use dyn_event::dyn_event;
use {
    crate::{AppError, AppResult, EventResult, NEXT_UPGRADE, TraceOption},
    grug_types::{Addr, BlockInfo, Config, EvtUpgrade, NextUpgrade, Storage},
};

pub fn do_upgrade(
    storage: &mut dyn Storage,
    block: BlockInfo,
    cfg: &Config,
    sender: Addr,
    upgrade: NextUpgrade,
    #[allow(unused_variables)] trace_opt: TraceOption,
) -> EventResult<EvtUpgrade> {
    let evt = EvtUpgrade {
        sender,
        height: upgrade.height,
        cargo_version: upgrade.cargo_version.clone(),
    };

    match _do_upgrade(storage, block, cfg, sender, upgrade.clone()) {
        Ok(_) => {
            #[cfg(feature = "tracing")]
            {
                dyn_event!(
                    trace_opt.ok_level.into(),
                    height = upgrade.height,
                    cargo_version = upgrade.cargo_version,
                    "Chain upgrade scheduled"
                );
            }

            EventResult::Ok(evt)
        },
        Err(err) => {
            #[cfg(feature = "tracing")]
            {
                dyn_event!(trace_opt.error_level.into(), %err, "Failed to schedule chain upgrade");
            }

            EventResult::err(evt, err)
        },
    }
}

fn _do_upgrade(
    storage: &mut dyn Storage,
    block: BlockInfo,
    cfg: &Config,
    sender: Addr,
    upgrade: NextUpgrade,
) -> AppResult<()> {
    // Only the chain owner can schedule upgrades.
    if sender != cfg.owner {
        return Err(AppError::not_owner(sender, cfg.owner));
    }

    // The upgrade height must be in the future.
    if upgrade.height <= block.height {
        return Err(AppError::upgrade_height_not_in_future(
            block.height,
            upgrade.height,
        ));
    }

    // Save the planned upgrade in storage.
    //
    // Note: this overwrites existing upgrade plan, if any. Since this is an
    // owner-only function, we trust the owner knows what he's doing.
    //
    // TODO: consume gas
    NEXT_UPGRADE.save(storage, &upgrade)?;

    Ok(())
}
