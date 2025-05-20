use {
    crate::{
        CONFIG, INBOUNDS, NEXT_OUTBOUND_ID, OUTBOUND_QUEUE, OUTBOUNDS, PROCESSED_UTXOS, SIGNATURES,
        UTXOS,
    },
    anyhow::ensure,
    corepc_client::bitcoin::Address,
    dango_types::{
        bank,
        bitcoin::{
            BitcoinAddress, BitcoinSignature, DENOM, ExecuteMsg, INPUT_SIZE, InboundConfirmed,
            InstantiateMsg, OUTPUT_SIZE, OVERHEAD_SIZE, OutboundConfirmed, OutboundRequested,
            Transaction, Vout,
        },
        taxman::{self, FeeType},
    },
    grug::{
        Addr, Coin, Coins, Empty, Hash256, Message, MutableCtx, Number, NumberConst, Order,
        QuerierExt as _, Response, StdResult, SudoCtx, Uint128, btree_map,
    },
    std::{collections::BTreeMap, str::FromStr},
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> anyhow::Result<Response> {
    ensure!(
        msg.config.threshold as usize <= msg.config.guardians.len(),
        "threshold ({}) cannot be greater than guardian set size ({})",
        msg.config.threshold,
        msg.config.guardians.len()
    );

    // Validate the vault address.
    Address::from_str(&msg.config.vault.to_string())?.require_network(msg.config.network)?;

    CONFIG.save(ctx.storage, &msg.config)?;

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn execute(ctx: MutableCtx, msg: ExecuteMsg) -> anyhow::Result<Response> {
    match msg {
        ExecuteMsg::UpdateConfig {
            sats_per_vbyte,
            outbound_fee,
            outbound_strategy,
        } => update_config(ctx, sats_per_vbyte, outbound_fee, outbound_strategy),
        ExecuteMsg::ObserveInbound {
            transaction_hash,
            vout,
            amount,
            recipient,
        } => observe_inbound(ctx, transaction_hash, vout, amount, recipient),
        ExecuteMsg::Withdraw { recipient } => withdraw(ctx, recipient),
        ExecuteMsg::AuthorizeOutbound { id, signatures } => authorize_outbound(ctx, id, signatures),
    }
}

fn update_config(
    ctx: MutableCtx,
    sats_per_vbyte: Option<Uint128>,
    outbound_fee: Option<Uint128>,
    outbound_strategy: Option<Order>,
) -> anyhow::Result<Response> {
    ensure!(
        ctx.sender == ctx.querier.query_owner()?,
        "you don't have the right, O you don't have the right"
    );

    CONFIG.update(ctx.storage, |mut cfg| -> StdResult<_> {
        if let Some(sats_per_vbyte) = sats_per_vbyte {
            cfg.sats_per_vbyte = sats_per_vbyte;
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
    vout: Vout,
    amount: Uint128,
    recipient: Option<Addr>,
) -> anyhow::Result<Response> {
    let cfg = CONFIG.load(ctx.storage)?;

    ensure!(
        cfg.guardians.contains(&ctx.sender),
        "you don't have the right, O you don't have the right"
    );

    ensure!(
        !PROCESSED_UTXOS.has(ctx.storage, (hash, vout)),
        "transaction `{hash}` already exists in UTXO set"
    );

    let inbound = (hash, vout, amount, recipient);
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
        PROCESSED_UTXOS.insert(ctx.storage, (hash, vout))?;
        UTXOS.save(ctx.storage, (amount, hash, vout), &Empty {})?;
        INBOUNDS.remove(ctx.storage, inbound);

        let maybe_msg = if let Some(recipient) = recipient {
            let bank = ctx.querier.query_bank()?;
            Some(Message::execute(
                bank,
                &bank::ExecuteMsg::Mint {
                    to: recipient,
                    coins: Coin::new(DENOM.clone(), amount)?.into(),
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

    // Validate the address.
    Address::from_str(&recipient.to_string())?.require_network(cfg.network)?;

    ensure!(
        recipient != cfg.vault,
        "cannot withdraw to the vault address"
    );

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
                    coins: Coin::new(coin.denom.clone(), amount_after_fee)?.into(),
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

    // Calculate the number of outputs, adding 1 (vault) for the change.
    let n_outupt = outputs.len() + 1;

    // Sum up the total outbound amount.
    let withdraw_amount = outputs
        .iter()
        .try_fold(Uint128::ZERO, |total, (_, amount)| {
            total.checked_add(*amount)
        })?;

    // Choose the UTXOs as inputs for the outbound transaction.
    let mut inputs = BTreeMap::new();
    let mut sum = Uint128::ZERO;

    // The size of the transaction is calculated as:
    // size = overhead + n_input * input_size + n_output * output_size
    // and the fee = size * sats_per_vbyte
    let mut fee =
        (OVERHEAD_SIZE + Uint128::new(n_outupt as u128) * OUTPUT_SIZE) * cfg.sats_per_vbyte;

    for res in UTXOS.keys(ctx.storage, None, None, cfg.outbound_strategy) {
        if sum >= withdraw_amount + fee {
            break;
        }

        let (amount, hash, vout) = res?;

        inputs.insert((hash, vout), amount);
        sum.checked_add_assign(amount)?;

        fee += INPUT_SIZE * cfg.sats_per_vbyte;
    }

    // Total amount of BTC needed for this tx.
    let total = withdraw_amount + fee;

    // If there's excess input, send the excess back to the vault.
    if sum > total {
        outputs.insert(cfg.vault, sum - total);
    }

    // Delete the chosen UTXOs.
    for ((hash, vout), amount) in &inputs {
        UTXOS.remove(ctx.storage, (*amount, *hash, *vout))?;
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
    signatures: Vec<BitcoinSignature>,
) -> anyhow::Result<Response> {
    let cfg = CONFIG.load(ctx.storage)?;

    ensure!(
        cfg.guardians.contains(&ctx.sender),
        "you don't have the right, O you don't have the right"
    );

    let transaction = OUTBOUNDS.load(ctx.storage, id)?;

    ensure!(
        transaction.inputs.len() == signatures.len(),
        "transaction `{id}` has {} inputs, but {} signatures were provided",
        transaction.inputs.len(),
        signatures.len()
    );

    let cumulative_signatures =
        SIGNATURES.may_update(ctx.storage, id, |cumulative_signatures| {
            let mut cumulative_signatures = cumulative_signatures.unwrap_or_default();

            ensure!(
                cumulative_signatures
                    .insert(ctx.sender, signatures)
                    .is_none(),
                "you've already signed transaction `{id}`"
            );

            Ok(cumulative_signatures)
        })?;

    Ok(
        Response::new().may_add_event(
            if cumulative_signatures.len() >= cfg.threshold as usize {
                Some(OutboundConfirmed {
                    id,
                    transaction: OUTBOUNDS.load(ctx.storage, id)?,
                    signatures: cumulative_signatures,
                })
            } else {
                None
            },
        )?,
    )
}
