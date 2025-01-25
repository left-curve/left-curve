use {
    crate::{MAILBOX, STORAGE_LOCATIONS},
    grug::{Addr, HexByteArray, ImmutableCtx, Json, JsonSerExt, Order, StdResult, StorageQuerier},
    hyperlane_types::{mailbox::Domain, va::QueryMsg},
    std::collections::{BTreeMap, BTreeSet},
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> StdResult<Json> {
    match msg {
        QueryMsg::AnnounceStorageLocations { validators } => {
            let res = get_announce(ctx, validators)?;
            res.to_json_value()
        },
        QueryMsg::AnnouncedValidators {} => {
            let res = get_validators(ctx)?;
            res.to_json_value()
        },
        QueryMsg::Mailbox {} => {
            let res = get_mailbox(ctx)?;
            res.to_json_value()
        },
        QueryMsg::LocalDomain {} => {
            let res = get_local_domain(ctx)?;
            res.to_json_value()
        },
    }
}

fn get_announce(
    ctx: ImmutableCtx,
    validators: BTreeSet<HexByteArray<20>>,
) -> StdResult<BTreeMap<HexByteArray<20>, BTreeSet<String>>> {
    let storage_locations = validators
        .into_iter()
        .map(|v| {
            let storage_locations = STORAGE_LOCATIONS
                .may_load(ctx.storage, v.to_vec().try_into().unwrap())?
                .unwrap_or_default();
            Ok((v, storage_locations))
        })
        .collect::<StdResult<_>>()?;

    Ok(storage_locations)
}

fn get_validators(ctx: ImmutableCtx) -> StdResult<BTreeSet<HexByteArray<20>>> {
    let validators = STORAGE_LOCATIONS
        .keys(ctx.storage, None, None, Order::Ascending)
        .collect::<StdResult<_>>()?;

    Ok(validators)
}

fn get_mailbox(ctx: ImmutableCtx) -> StdResult<Addr> {
    MAILBOX.load(ctx.storage)
}

fn get_local_domain(ctx: ImmutableCtx) -> StdResult<Domain> {
    Ok(ctx
        .querier
        .query_wasm_path(MAILBOX.load(ctx.storage)?, hyperlane_mailbox::CONFIG.path())?
        .local_domain)
}
