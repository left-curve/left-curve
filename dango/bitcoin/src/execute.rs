use {
    crate::{CONFIG, INBOUNDS, NEXT_OUTBOUND_ID, OUTBOUND_QUEUE, OUTBOUNDS, SIGNATURES, UTXOS},
    anyhow::ensure,
    dango_types::{
        bank,
        bitcoin::{
            BitcoinAddress, BitcoinSignature, DENOM, ExecuteMsg, InboundConfirmed, InstantiateMsg,
            OutboundConfirmed, OutboundRequested, Transaction,
        },
        taxman::{self, FeeType},
    },
    grug::{
        Addr, Coins, Empty, Hash256, Message, MutableCtx, Number, NumberConst, Order,
        QuerierExt as _, Response, StdResult, SudoCtx, Uint128, btree_map,
    },
    std::collections::BTreeMap,
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> anyhow::Result<Response> {
    ensure!(
        msg.config.threshold as usize <= msg.config.guardians.len(),
        "threshold ({}) cannot be greater than guardian set size ({})",
        msg.config.threshold,
        msg.config.guardians.len()
    );

    CONFIG.save(ctx.storage, &msg.config)?;

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn execute(ctx: MutableCtx, msg: ExecuteMsg) -> anyhow::Result<Response> {
    match msg {
        ExecuteMsg::UpdateConfig {
            outbound_gas,
            outbound_fee,
            outbound_strategy,
        } => update_config(ctx, outbound_gas, outbound_fee, outbound_strategy),
        ExecuteMsg::ObserveInbound {
            transaction_hash,
            amount,
            recipient,
        } => observe_inbound(ctx, transaction_hash, amount, recipient),
        ExecuteMsg::Withdraw { recipient } => withdraw(ctx, recipient),
        ExecuteMsg::AuthorizeOutbound { id, signature } => authorize_outbound(ctx, id, signature),
    }
}

fn update_config(
    ctx: MutableCtx,
    outbound_gas: Option<Uint128>,
    outbound_fee: Option<Uint128>,
    outbound_strategy: Option<Order>,
) -> anyhow::Result<Response> {
    ensure!(
        ctx.sender == ctx.querier.query_owner()?,
        "you don't have the right, O you don't have the right"
    );

    CONFIG.update(ctx.storage, |mut cfg| -> StdResult<_> {
        if let Some(outbound_gas) = outbound_gas {
            cfg.outbound_gas = outbound_gas;
        }

        if let Some(outbound_fee) = outbound_fee {
            cfg.outbound_fee = outbound_fee;
        }

        if let Some(outbound_strategy) = outbound_strategy {
            cfg.outbound_strategy = outbound_strategy;
        }

        Ok(cfg)
    })?;

    Ok(Response::new())
}

fn observe_inbound(
    ctx: MutableCtx,
    hash: Hash256,
    amount: Uint128,
    recipient: Option<Addr>,
) -> anyhow::Result<Response> {
    let cfg = CONFIG.load(ctx.storage)?;

    ensure!(
        cfg.guardians.contains(&ctx.sender),
        "you don't have the right, O you don't have the right"
    );

    ensure!(
        !UTXOS.idx.transaction_hash.has(ctx.storage, hash),
        "transaction `{hash}` already exists in UTXO set"
    );

    let inbound = (hash, amount, recipient);
    let mut voters = INBOUNDS.may_load(ctx.storage, inbound)?.unwrap_or_default();

    ensure!(
        voters.insert(ctx.sender),
        "you've already voted for transaction `{hash}`"
    );

    // If a threshold number of votes has been reached,
    //
    // 1. Mint synthetic Bitcoin tokens to the recipient, if presents.
    // 2. Add the transaction to the UTXO set.
    //
    // Otherwise, simply save the voters set, then we're done.
    let (maybe_msg, maybe_event) = if voters.len() >= cfg.threshold as usize {
        UTXOS.save(ctx.storage, (amount, hash), &Empty {})?;
        INBOUNDS.remove(ctx.storage, inbound);

        let maybe_msg = if let Some(recipient) = recipient {
            let bank = ctx.querier.query_bank()?;
            Some(Message::execute(
                bank,
                &bank::ExecuteMsg::Mint {
                    to: recipient,
                    denom: DENOM.clone(),
                    amount,
                },
                Coins::new(),
            )?)
        } else {
            None
        };

        let event = InboundConfirmed {
            transaction_hash: hash,
            amount,
            recipient,
        };

        (maybe_msg, Some(event))
    } else {
        INBOUNDS.save(ctx.storage, inbound, &voters)?;

        (None, None)
    };

    Ok(Response::new()
        .may_add_message(maybe_msg)
        .may_add_event(maybe_event)?)
}

fn withdraw(ctx: MutableCtx, recipient: BitcoinAddress) -> anyhow::Result<Response> {
    let cfg = CONFIG.load(ctx.storage)?;
    let coin = ctx.funds.into_one_coin_of_denom(&DENOM)?;

    ensure!(
        coin.amount > cfg.outbound_fee,
        "withdrawal amount ({}) must be greater than outbound fee ({})",
        coin.amount,
        cfg.outbound_fee
    );

    let amount_after_fee = coin.amount - cfg.outbound_fee;

    OUTBOUND_QUEUE.may_update(ctx.storage, recipient, |outbound| -> StdResult<_> {
        Ok(outbound.unwrap_or_default().checked_add(amount_after_fee)?)
    })?;

    Ok(Response::new()
        .add_message({
            let bank = ctx.querier.query_bank()?;
            Message::execute(
                bank,
                &bank::ExecuteMsg::Burn {
                    from: ctx.contract,
                    denom: coin.denom.clone(),
                    amount: amount_after_fee,
                },
                Coins::new(),
            )?
        })
        .add_message({
            let taxman = ctx.querier.query_taxman()?;
            let fee = Coins::one(coin.denom, cfg.outbound_fee)?;
            Message::execute(
                taxman,
                &taxman::ExecuteMsg::Pay {
                    ty: FeeType::Withdraw,
                    payments: btree_map! { ctx.sender => fee.clone() },
                },
                fee,
            )?
        }))
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn cron_execute(ctx: SudoCtx) -> StdResult<Response> {
    let cfg = CONFIG.load(ctx.storage)?;

    // Take the pending outbound transfers from the storage.
    let mut outputs = OUTBOUND_QUEUE.drain(ctx.storage, None, None)?;

    // If there's no pending outbound transfers, nothing to do.
    if outputs.is_empty() {
        return Ok(Response::new());
    }

    // Sum up the total outbound amount.
    // Make sure to include the Bitcoin gas fee.
    let total = outputs
        .iter()
        .try_fold(Uint128::ZERO, |total, (_, amount)| {
            total.checked_add(*amount)
        })?
        .checked_add(cfg.outbound_gas)?;

    // Choose the UTXOs as inputs for the outbound transaction.
    let mut inputs = BTreeMap::new();
    let mut sum = Uint128::ZERO;

    for res in UTXOS.keys(ctx.storage, None, None, cfg.outbound_strategy) {
        if sum >= total {
            break;
        }

        let (amount, hash) = res?;

        inputs.insert(hash, amount);
        sum.checked_add_assign(amount)?;
    }

    // Delete the chosen UTXOs.
    for (hash, amount) in &inputs {
        UTXOS.remove(ctx.storage, (*amount, *hash))?;
    }

    // If there's excess input, send the excess back to the vault.
    if total > sum {
        outputs.insert(cfg.vault, total - sum);
    }

    let (id, _) = NEXT_OUTBOUND_ID.increment(ctx.storage)?;
    let transaction = Transaction {
        inputs,
        outputs,
        fee: cfg.outbound_fee,
    };

    // Save the outbound transaction.
    OUTBOUNDS.save(ctx.storage, id, &transaction)?;

    Response::new().add_event(OutboundRequested { id, transaction })
}

fn authorize_outbound(
    ctx: MutableCtx,
    id: u32,
    signature: BitcoinSignature,
) -> anyhow::Result<Response> {
    let cfg = CONFIG.load(ctx.storage)?;

    ensure!(
        cfg.guardians.contains(&ctx.sender),
        "you don't have the right, O you don't have the right"
    );

    let signatures = SIGNATURES.may_update(ctx.storage, id, |signatures| {
        let mut signatures = signatures.unwrap_or_default();

        ensure!(
            signatures.insert(ctx.sender, signature).is_none(),
            "you've already signed transaction `{id}`"
        );

        Ok(signatures)
    })?;

    Ok(
        Response::new().may_add_event(if signatures.len() >= cfg.threshold as usize {
            Some(OutboundConfirmed {
                id,
                transaction: OUTBOUNDS.load(ctx.storage, id)?,
                signatures,
            })
        } else {
            None
        })?,
    )
}
