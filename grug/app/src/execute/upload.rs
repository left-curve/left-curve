#[cfg(feature = "tracing")]
use dyn_event::dyn_event;
use {
    crate::{
        AppError, AppResult, CODES, CONFIG, EventResult, GasTracker, MeteredItem, MeteredMap,
        TraceOption, has_permission,
    },
    grug_types::{
        Addr, BlockInfo, Code, CodeStatus, EvtUpload, Hash256, HashExt, MsgUpload, Storage,
    },
};

pub fn do_upload(
    storage: &mut dyn Storage,
    gas_tracker: GasTracker,
    block: BlockInfo,
    uploader: Addr,
    msg: MsgUpload,
    trace_opt: TraceOption,
) -> EventResult<EvtUpload> {
    let code_hash = msg.code.hash256();

    let evt = EvtUpload {
        sender: uploader,
        code_hash,
    };

    match _do_upload(storage, gas_tracker, block, uploader, msg, code_hash) {
        Ok(_) => {
            #[cfg(feature = "tracing")]
            dyn_event!(
                trace_opt.ok_level,
                code_hash = code_hash.to_string(),
                "Uploaded code"
            );

            EventResult::Ok(evt)
        },
        Err(err) => {
            #[cfg(feature = "tracing")]
            dyn_event!(
                trace_opt.error_level,
                err = err.to_string(),
                "Failed to upload code"
            );

            EventResult::err(evt, err)
        },
    }
}

// Return the hash of the code that is stored, for logging purpose.
fn _do_upload(
    storage: &mut dyn Storage,
    gas_tracker: GasTracker,
    block: BlockInfo,
    uploader: Addr,
    msg: MsgUpload,
    code_hash: Hash256,
) -> AppResult<()> {
    // Make sure the user has the permission to upload contracts
    let cfg = CONFIG.load_with_gas(storage, gas_tracker.clone())?;

    if !has_permission(&cfg.permissions.upload, cfg.owner, uploader) {
        return Err(AppError::Unauthorized);
    }

    if CODES.has_with_gas(storage, gas_tracker.clone(), code_hash)? {
        return Err(AppError::CodeExists { code_hash });
    }

    CODES.save_with_gas(storage, gas_tracker, code_hash, &Code {
        code: msg.code,
        status: CodeStatus::Orphaned {
            since: block.timestamp,
        },
    })?;

    Ok(())
}
