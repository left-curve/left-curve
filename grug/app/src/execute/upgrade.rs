#[cfg(feature = "tracing")]
use dyn_event::dyn_event;
use {
    crate::{
        AppError, AppResult, CONFIG, EventResult, GasTracker, MeteredItem, NEXT_UPGRADE,
        TraceOption,
    },
    grug_types::{Addr, BlockInfo, EvtUpgrade, NextUpgrade, Storage},
};

pub fn do_upgrade(
    storage: &mut dyn Storage,
    gas_tracker: GasTracker,
    block: BlockInfo,
    sender: Addr,
    upgrade: NextUpgrade,
    #[allow(unused_variables)] trace_opt: TraceOption,
) -> EventResult<EvtUpgrade> {
    let evt = EvtUpgrade {
        sender,
        height: upgrade.height,
        cargo_version: upgrade.cargo_version.clone(),
    };

    match _do_upgrade(storage, gas_tracker, block, sender, upgrade.clone()) {
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
    gas_tracker: GasTracker,
    block: BlockInfo,
    sender: Addr,
    upgrade: NextUpgrade,
) -> AppResult<()> {
    let cfg = CONFIG.load_with_gas(storage, gas_tracker)?;

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
