use {
    crate::{ALLOYS, DENOMS, FEES, OUTBOUND_QUOTAS, RATE_LIMITS, REVERSE_ALLOYS},
    anyhow::{anyhow, ensure},
    dango_types::{
        bank,
        taxman::{self, FeeType},
        token_minter::{
            self, ALLOY_SUBNAMESPACE, DestinationAddr, DestinationChain, ExecuteMsg,
            HookTransferRemote, InstantiateMsg, NAMESPACE, RateLimit, TransferRemote,
        },
    },
    grug::{
        Addr, Coin, Coins, Denom, Inner, IsZero, Message, MultiplyFraction, MutableCtx, Number,
        QuerierExt, Response, StdResult, SudoCtx, Uint128, btree_map, coins,
    },
    std::collections::BTreeMap,
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> StdResult<Response> {
    for (denom, (bridge_addr, fee)) in msg.denoms {
        // Save the denom and its corresponding bridge address and fee.
        DENOMS.save(ctx.storage, &denom, &bridge_addr)?;
        FEES.save(ctx.storage, &denom, &fee)?;
    }

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn execute(ctx: MutableCtx, msg: ExecuteMsg) -> anyhow::Result<Response> {
    match msg {
        ExecuteMsg::TransferRemote {
            destination_chain,
            recipient,
        } => transfer_remote(ctx, destination_chain, recipient),
        ExecuteMsg::ReceiveRemote { token, recipient } => receive_remote(ctx, token, recipient),
        ExecuteMsg::SetAlloy {
            underlying_denom,
            destination_chain,
            alloyed_denom,
        } => set_alloy(ctx, underlying_denom, destination_chain, alloyed_denom),
        ExecuteMsg::SetRateLimits(limits) => set_rate_limits(ctx, limits),
        ExecuteMsg::RegisterDenom {
            denom,
            bridge_addr,
            fee,
        } => register_denom(ctx, denom, bridge_addr, fee),
    }
}

fn register_denom(
    ctx: MutableCtx,
    denom: Denom,
    bridge: Addr,
    fee: Uint128,
) -> anyhow::Result<Response> {
    ensure!(
        ctx.sender == ctx.querier.query_owner()?,
        "you don't have the right, O you don't have the right"
    );

    DENOMS.save(ctx.storage, &denom, &bridge)?;
    FEES.save(ctx.storage, &denom, &fee)?;

    Ok(Response::new())
}

fn transfer_remote(
    ctx: MutableCtx,
    destination_chain: DestinationChain,
    recipient: DestinationAddr,
) -> anyhow::Result<Response> {
    ensure!(
        &recipient == &destination_chain,
        "recipient must be the same as destination chain"
    );

    let cfg = ctx.querier.query_config()?;
    let token = ctx.funds.into_one_coin()?;
    let bridge = DENOMS.load(ctx.storage, &token.denom)?;

    // Check if the token is alloyed.
    let (mut token, burn_alloy_msg) = if let Some(base_denom) =
        REVERSE_ALLOYS.may_load(ctx.storage, (&token.denom, &destination_chain))?
    {
        // Burn the alloy token.
        let burn_alloy_msg = Message::execute(
            cfg.bank,
            &bank::ExecuteMsg::Burn {
                from: ctx.contract,
                coins: coins! { token.denom.clone() => token.amount },
            },
            Coins::new(),
        )?;

        (Coin::new(base_denom, token.amount)?, Some(burn_alloy_msg))
    } else {
        (token, None)
    };

    let fee = FEES.load(ctx.storage, &token.denom)?;

    token.amount.checked_sub_assign(fee).map_err(|_| {
        anyhow!(
            "withdrawal amount not sufficient to cover fee: {} < {}",
            token.amount,
            fee
        )
    })?;

    // Check if the rate limit is reached.
    if let Some(mut quota) = OUTBOUND_QUOTAS.may_load(ctx.storage, &token.denom)? {
        quota.checked_sub_assign(token.amount).map_err(|_| {
            anyhow!(
                "withdrawal rate limit reached: {} < {}",
                quota,
                token.amount
            )
        })?;

        OUTBOUND_QUOTAS.save(ctx.storage, &token.denom, &quota)?;
    }

    // 1. Burn the alloy token, if the token being sent is an alloy token.
    // 2. If the token is collateral, escrow it (no need to do anything);
    //    otherwise (it's "synthetic"), burn it.
    //    We determine whether it's synthetic by checking whether its denom is
    //    under the `bri` namespace.
    // 3. Pay withdrawal fee to the taxman.
    // 4. Send hook to bridge contract.
    Ok(Response::new()
        .may_add_message(burn_alloy_msg)
        .may_add_message(if token.denom.namespace() == Some(&NAMESPACE) {
            Some(Message::execute(
                cfg.bank,
                &bank::ExecuteMsg::Burn {
                    from: ctx.contract,
                    coins: coins! { token.denom.clone() => token.amount },
                },
                Coins::new(),
            )?)
        } else {
            None
        })
        .may_add_message(if fee.is_non_zero() {
            Some(Message::execute(
                cfg.taxman,
                &taxman::ExecuteMsg::Pay {
                    ty: FeeType::Withdraw,
                    payments: btree_map! {
                        ctx.sender => coins! { token.denom.clone() => fee },
                    },
                },
                coins! { token.denom.clone() => fee },
            )?)
        } else {
            None
        })
        .add_message(Message::execute(
            bridge,
            &token_minter::BridgeHookMsg::HookTransferRemote(HookTransferRemote {
                token: token.clone(),
                destination_chain,
                recipient,
            }),
            Coins::new(),
        )?)
        .add_event(TransferRemote {
            sender: ctx.sender,
            destination_chain,
            recipient,
            token: token.denom,
            amount: token.amount,
        })?)
}

fn receive_remote(ctx: MutableCtx, token: Coin, recipient: Addr) -> anyhow::Result<Response> {
    let bank = ctx.querier.query_bank()?;
    // Check if the denom is alloyed.
    let (denom, mint_underlying_msg) =
        if let Some(alloy_denom) = ALLOYS.may_load(ctx.storage, &token.denom)? {
            // Mint the base denom to the wrapper.
            let msg = Message::execute(
                bank,
                &bank::ExecuteMsg::Mint {
                    to: ctx.contract,
                    coins: coins! { token.denom => token.amount },
                },
                Coins::new(),
            )?;
            (alloy_denom, Some(msg))
        } else {
            (token.denom, None)
        };

    // Increase the remaining outbound quota.
    if let Some(mut quota) = OUTBOUND_QUOTAS.may_load(ctx.storage, &denom)? {
        quota.checked_add_assign(token.amount)?;

        OUTBOUND_QUOTAS.save(ctx.storage, &denom, &quota)?;
    }

    Ok(Response::new()
        // If the denom is synthetic, then mint the token.
        // Otherwise, if it's a collateral, then release the collateral.
        .add_message(if denom.namespace() == Some(&NAMESPACE) {
            Message::execute(
                bank,
                &bank::ExecuteMsg::Mint {
                    to: recipient,
                    coins: coins! { denom.clone() => token.amount },
                },
                Coins::new(),
            )?
        } else {
            Message::transfer(recipient, Coin {
                denom: denom.clone(),
                amount: token.amount,
            })?
        })
        .may_add_message(mint_underlying_msg))
}

#[inline]
fn set_alloy(
    ctx: MutableCtx,
    underlying_denom: Denom,
    destination_chain: DestinationChain,
    alloyed_denom: Denom,
) -> anyhow::Result<Response> {
    ensure!(
        ctx.sender == ctx.querier.query_owner()?,
        "only chain owner can call `set_alloy`"
    );

    ensure!(
        alloyed_denom.starts_with(&[NAMESPACE.clone(), ALLOY_SUBNAMESPACE.clone()]),
        "alloyed denom must start with `{}/{}`",
        NAMESPACE.as_ref(),
        ALLOY_SUBNAMESPACE.as_ref()
    );

    ALLOYS.save(ctx.storage, &underlying_denom, &alloyed_denom)?;

    REVERSE_ALLOYS.save(
        ctx.storage,
        (&alloyed_denom, &destination_chain),
        &underlying_denom,
    )?;

    Ok(Response::new())
}

#[inline]
fn set_rate_limits(
    ctx: MutableCtx,
    limits: BTreeMap<Denom, RateLimit>,
) -> anyhow::Result<Response> {
    ensure!(
        ctx.sender == ctx.querier.query_owner()?,
        "only chain owner can call `set_rate_limits`"
    );

    RATE_LIMITS.save(ctx.storage, &limits)?;

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn cron_execute(ctx: SudoCtx) -> StdResult<Response> {
    // Clear the quotas for the previous 24-hour window.
    OUTBOUND_QUOTAS.clear(ctx.storage, None, None);

    // Set quotes for the next 24-hour window.
    for (denom, limit) in RATE_LIMITS.load(ctx.storage)? {
        let supply = ctx.querier.query_supply(denom.clone())?;
        let quota = supply.checked_mul_dec_floor(limit.into_inner())?;
        OUTBOUND_QUOTAS.save(ctx.storage, &denom, &quota)?;
    }

    Ok(Response::new())
}
