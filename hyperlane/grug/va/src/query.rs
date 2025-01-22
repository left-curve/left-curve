use {
    crate::{LOCAL_DOMAIN, MAILBOX, STORAGE_LOCATIONS, VALIDATORS},
    grug::{HexBinary, ImmutableCtx, Json, JsonSerExt, Order, StdResult},
    hyperlane_types::va::{
        GetAnnounceStorageLocationsResponse, GetAnnouncedValidatorsResponse, LocalDomainResponse,
        MailboxResponse, QueryMsg,
    },
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> StdResult<Json> {
    match msg {
        QueryMsg::GetAnnounceStorageLocations { validators } => {
            let res = get_announce(ctx, validators)?;
            res.to_json_value()
        },
        QueryMsg::GetAnnouncedValidators {} => {
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
    validators: Vec<HexBinary>,
) -> StdResult<GetAnnounceStorageLocationsResponse> {
    let storage_locations = validators
        .into_iter()
        .map(|v| {
            let storage_locations = STORAGE_LOCATIONS
                .may_load(ctx.storage, v.to_vec().try_into().unwrap())?
                .unwrap_or_default();
            Ok((v.to_string(), storage_locations))
        })
        .collect::<StdResult<Vec<_>>>()?;

    Ok(GetAnnounceStorageLocationsResponse { storage_locations })
}

fn get_validators(ctx: ImmutableCtx) -> StdResult<GetAnnouncedValidatorsResponse> {
    let validators = VALIDATORS
        .range(ctx.storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()?;

    Ok(GetAnnouncedValidatorsResponse { validators })
}

fn get_mailbox(ctx: ImmutableCtx) -> StdResult<MailboxResponse> {
    Ok(MailboxResponse {
        mailbox: MAILBOX.load(ctx.storage)?.to_string(),
    })
}

fn get_local_domain(ctx: ImmutableCtx) -> StdResult<LocalDomainResponse> {
    Ok(LocalDomainResponse {
        local_domain: LOCAL_DOMAIN.load(ctx.storage)?,
    })
}
