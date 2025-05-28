use {
    crate::{
        CONFIG, INBOUNDS, NEXT_OUTBOUND_ID, OUTBOUND_QUEUE, OUTBOUNDS, PROCESSED_UTXOS, SIGNATURES,
        UTXOS,
    },
    anyhow::{bail, ensure},
    corepc_client::bitcoin::Address,
    dango_types::{
        DangoQuerier,
        bitcoin::{
            BitcoinSignature, ExecuteMsg, INPUT_SIZE, InboundConfirmed, InstantiateMsg, Network,
            OUTPUT_SIZE, OVERHEAD_SIZE, OutboundConfirmed, OutboundRequested, Transaction, Vout,
        },
        gateway::{
            self, Remote,
            bridge::{BridgeMsg, TransferRemoteRequest},
        },
    },
    grug::{
        Addr, Coins, Empty, Hash256, Message, MutableCtx, Number, NumberConst, Order,
        QuerierExt as _, Response, StdResult, SudoCtx, Uint128,
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

    // Ensure the vault address is valid.
    check_bitcoin_address(&msg.config.vault, msg.config.network)?;

    CONFIG.save(ctx.storage, &msg.config)?;

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn execute(ctx: MutableCtx, msg: ExecuteMsg) -> anyhow::Result<Response> {
    match msg {
        ExecuteMsg::UpdateConfig {
            sats_per_vbyte,
            outbound_strategy,
        } => update_config(ctx, sats_per_vbyte, outbound_strategy),
        ExecuteMsg::ObserveInbound {
            transaction_hash,
            vout,
            amount,
            recipient,
        } => observe_inbound(ctx, transaction_hash, vout, amount, recipient),
        ExecuteMsg::Bridge(BridgeMsg::TransferRemote { req, amount }) => {
            transfer_remote(ctx, req, amount)
        },
        ExecuteMsg::AuthorizeOutbound { id, signatures } => authorize_outbound(ctx, id, signatures),
    }
}

fn update_config(
    ctx: MutableCtx,
    sats_per_vbyte: Option<Uint128>,
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
        amount >= cfg.minimum_deposit,
        "minimum deposit not met: {} < {}",
        amount,
        cfg.minimum_deposit
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
            let gateway = ctx.querier.query_gateway()?;
            Some(Message::execute(
                gateway,
                &gateway::ExecuteMsg::ReceiveRemote {
                    remote: Remote::Bitcoin,
                    amount,
                    recipient,
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

fn transfer_remote(
    ctx: MutableCtx,
    req: TransferRemoteRequest,
    amount: Uint128,
) -> anyhow::Result<Response> {
    ensure!(
        ctx.sender == ctx.querier.query_gateway()?,
        "only gateway can call `transfer_remote`"
    );

    let TransferRemoteRequest::Bitcoin { recipient } = req else {
        bail!("incorrect TransferRemoteRequest type! expected: Bitcoin, found: {req:?}");
    };

    let cfg = CONFIG.load(ctx.storage)?;

    // Ensure the recipient address is valid.
    check_bitcoin_address(&recipient, cfg.network)?;

    ensure!(
        recipient != cfg.vault,
        "cannot withdraw to the vault address"
    );

    OUTBOUND_QUEUE.may_update(ctx.storage, recipient, |outbound| -> StdResult<_> {
        Ok(outbound.unwrap_or_default().checked_add(amount)?)
    })?;

    Ok(Response::new())
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
        fee,
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

/// Ensure the given Bitcoin address is valid for the specified network.
fn check_bitcoin_address(address: &str, network: Network) -> anyhow::Result<()> {
    Address::from_str(address)
        .map_err(|_| anyhow::anyhow!("address `{}` is not a valid Bitcoin address", address,))?
        .require_network(network)
        .map_err(|_| {
            anyhow::anyhow!(
                "address `{}` is not a valid Bitcoin address for network `{}`",
                address,
                network
            )
        })?;

    Ok(())
}
