use {
    crate::VALIDATOR_SETS,
    anyhow::ensure,
    grug::{HexByteArray, MutableCtx, QuerierExt, Response, StdResult},
    hyperlane_types::{
        isms::multisig::{ExecuteMsg, InstantiateMsg, ValidatorSet},
        mailbox::Domain,
    },
    std::collections::BTreeSet,
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> StdResult<Response> {
    for (domain, validator_set) in msg.validator_sets {
        VALIDATOR_SETS.save(ctx.storage, domain, &validator_set)?;
    }

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn execute(ctx: MutableCtx, msg: ExecuteMsg) -> anyhow::Result<Response> {
    match msg {
        ExecuteMsg::SetValidators {
            domain,
            threshold,
            validators,
        } => set_validators(ctx, domain, threshold, validators),
    }
}

#[inline]
fn set_validators(
    ctx: MutableCtx,
    domain: Domain,
    threshold: u32,
    validators: BTreeSet<HexByteArray<20>>,
) -> anyhow::Result<Response> {
    ensure!(
        ctx.sender == ctx.querier.query_owner()?,
        "only the chain owner can call `set_validators`"
    );

    ensure!(
        validators.len() >= threshold as usize,
        "not enough validators! threshold: {}, validators: {}",
        threshold,
        validators.len()
    );

    VALIDATOR_SETS.save(ctx.storage, domain, &ValidatorSet {
        threshold,
        validators,
    })?;

    Ok(Response::new())
}
