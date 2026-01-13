use {
    crate::{REFEREE, REFEREE_COUNT},
    dango_types::{
        DangoQuerier,
        account_factory::{QueryAccountRequest, UserIndex},
        referral::{ExecuteMsg, InstantiateMsg},
    },
    grug::{MutableCtx, QuerierExt, Response},
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn instantiate(_ctx: MutableCtx, _msg: InstantiateMsg) -> anyhow::Result<Response> {
    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn execute(ctx: MutableCtx, msg: ExecuteMsg) -> anyhow::Result<Response> {
    match msg {
        ExecuteMsg::Referral { referrer_index } => register_referral(ctx, referrer_index)?,
    }
    Ok(Response::new())
}

fn register_referral(ctx: MutableCtx, referrer_index: UserIndex) -> anyhow::Result<()> {
    // Retrieve the UserIndex of the sender.
    let account_factory = ctx.querier.query_account_factory()?;

    let user_index = ctx
        .querier
        .query_wasm_smart(account_factory, QueryAccountRequest {
            address: ctx.sender.clone(),
        })?
        .params
        .into_single()
        .owner;

    // Store the referral relationship.
    REFEREE.may_update(ctx.storage, user_index, |maybe_referrer| {
        if let Some(_) = maybe_referrer {
            anyhow::bail!("referral already registered for this user");
        }
        Ok(referrer_index)
    })?;

    // Increment the referrer's referee count.
    REFEREE_COUNT.may_update(
        ctx.storage,
        referrer_index,
        |maybe_count| -> anyhow::Result<u32> {
            let count = maybe_count.unwrap_or(0);
            Ok(count + 1)
        },
    )?;

    Ok(())
}
