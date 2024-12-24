use {
    crate::VALIDATOR_SETS,
    grug::{Bound, HexBinary, ImmutableCtx, Json, JsonSerExt, Order, StdResult},
    hyperlane_types::{
        ism::{QueryMsg, ValidatorSet},
        mailbox::Domain,
    },
    std::collections::BTreeMap,
};

const DEFAULT_PAGE_LIMIT: u32 = 30;

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> StdResult<Json> {
    match msg {
        QueryMsg::ValidatorSet { domain } => {
            let res = query_validaor_set(ctx, domain)?;
            res.to_json_value()
        },
        QueryMsg::ValidatorSets { start_after, limit } => {
            let res = query_validator_sets(ctx, start_after, limit)?;
            res.to_json_value()
        },
        QueryMsg::Verify {
            raw_message,
            metadata,
        } => {
            verify(ctx, raw_message, metadata)?;
            ().to_json_value()
        },
    }
}

#[inline]
fn query_validaor_set(ctx: ImmutableCtx, domain: Domain) -> StdResult<ValidatorSet> {
    VALIDATOR_SETS.load(ctx.storage, domain)
}

#[inline]
fn query_validator_sets(
    ctx: ImmutableCtx,
    start_after: Option<Domain>,
    limit: Option<u32>,
) -> StdResult<BTreeMap<Domain, ValidatorSet>> {
    let start = start_after.map(Bound::Exclusive);
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT);

    VALIDATOR_SETS
        .range(ctx.storage, start, None, Order::Ascending)
        .take(limit as usize)
        .collect()
}

#[inline]
fn verify(_ctx: ImmutableCtx, _raw_message: HexBinary, _metadata: HexBinary) -> StdResult<()> {
    // TODO

    Ok(())
}
