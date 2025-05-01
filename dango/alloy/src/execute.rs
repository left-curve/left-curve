use {
    crate::{ALLOYED_TO_UNDERLYING, UNDERLYING_TO_ALLOYED},
    anyhow::{anyhow, ensure},
    dango_types::{
        alloy::{Action, ExecuteMsg, InstantiateMsg, NAMESPACE},
        bank,
    },
    grug::{Coins, Denom, Message, MutableCtx, Part, QuerierExt as _, Response},
    std::collections::BTreeMap,
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> anyhow::Result<Response> {
    set_mapping(ctx, msg.mapping)
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn execute(ctx: MutableCtx, msg: ExecuteMsg) -> anyhow::Result<Response> {
    match msg {
        ExecuteMsg::SetMapping(mapping) => {
            ensure!(
                ctx.sender == ctx.querier.query_owner()?,
                "you don't have the right, O you don't have the right"
            );

            set_mapping(ctx, mapping)
        },
        ExecuteMsg::Alloy { and_then } => alloy(ctx, and_then),
        ExecuteMsg::Dealloy { and_then } => dealloy(ctx, and_then),
    }
}

fn set_mapping(ctx: MutableCtx, mapping: BTreeMap<Denom, Part>) -> anyhow::Result<Response> {
    for (underlying_denom, alloyed_subdenom) in mapping {
        let alloyed_denom = Denom::from_parts([NAMESPACE.clone(), alloyed_subdenom])?;

        UNDERLYING_TO_ALLOYED.save(ctx.storage, &underlying_denom, &alloyed_denom)?;
        ALLOYED_TO_UNDERLYING.save(ctx.storage, &alloyed_denom, &underlying_denom)?;
    }

    Ok(Response::new())
}

fn alloy(ctx: MutableCtx, and_then: Option<Action>) -> anyhow::Result<Response> {
    // Convert the underlying denoms to alloyed denoms.
    let alloyed_coins = ctx
        .funds
        .iter()
        .map(|underlying_coin| -> anyhow::Result<_> {
            let alloyed_denom = UNDERLYING_TO_ALLOYED
                .may_load(ctx.storage, &underlying_coin.denom)?
                .ok_or_else(|| {
                    anyhow!(
                        "no alloyed denom found for underlying denom `{}`",
                        underlying_coin.denom
                    )
                })?;

            Ok((alloyed_denom, *underlying_coin.amount))
        })
        .collect::<Result<BTreeMap<_, _>, _>>()
        .map(Coins::new_unchecked)?; // Unchecked is ok here because we know the denom is valid and amount is non-zero.

    // 1. Mint the alloyed tokens.
    // 2. Do the `and_then` action, if specified.
    Ok(Response::new()
        .add_message({
            let bank = ctx.querier.query_bank()?;
            Message::execute(
                bank,
                &bank::ExecuteMsg::Mint {
                    // Mint to the alloy contract itself if there's an `and_then`
                    // action to do. Otherwise, just mint to the caller.
                    to: if and_then.is_some() {
                        ctx.contract
                    } else {
                        ctx.sender
                    },
                    coins: alloyed_coins.clone(),
                },
                Coins::new(),
            )?
        })
        .may_add_message(if let Some(action) = and_then {
            Some(action.into_message(alloyed_coins))
        } else {
            None
        }))
}

fn dealloy(ctx: MutableCtx, and_then: Option<Action>) -> anyhow::Result<Response> {
    // Convert the alloyed denoms to underlying denoms.
    let underlying_coins = ctx
        .funds
        .iter()
        .map(|alloyed_coin| -> anyhow::Result<_> {
            let underlying_denom = ALLOYED_TO_UNDERLYING
                .may_load(ctx.storage, &alloyed_coin.denom)?
                .ok_or_else(|| {
                    anyhow!(
                        "no underlying denom found for alloyed denom `{}`",
                        alloyed_coin.denom
                    )
                })?;

            Ok((underlying_denom, *alloyed_coin.amount))
        })
        .collect::<Result<BTreeMap<_, _>, _>>()
        .map(Coins::new_unchecked)?;

    // 1. Burn the received alloyed coins.
    // 2. Do the `and_then` action, if specified; otherwise, refund the underlying
    //    coins to the caller.
    Ok(Response::new()
        .add_message({
            let bank = ctx.querier.query_bank()?;
            Message::execute(
                bank,
                &bank::ExecuteMsg::Burn {
                    from: ctx.contract,
                    coins: ctx.funds.clone(),
                },
                Coins::new(),
            )?
        })
        .add_message(if let Some(action) = and_then {
            action.into_message(underlying_coins)
        } else {
            Message::transfer(ctx.sender, underlying_coins)?
        }))
}
