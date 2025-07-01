use {
    crate::{
        CONFIG, INBOUNDS, OUTBOUND_ID, OUTBOUND_QUEUE, OUTBOUNDS, PROCESSED_UTXOS, SIGNATURES,
        UTXOS,
    },
    anyhow::{anyhow, bail, ensure},
    corepc_client::bitcoin::{
        Address, Amount, EcdsaSighashType,
        key::Secp256k1,
        secp256k1::{self, PublicKey, ecdsa::Signature},
        sighash::SighashCache,
    },
    dango_types::{
        DangoQuerier,
        bitcoin::{
            BitcoinSignature, ExecuteMsg, INPUT_SIGNATURES_OVERHEAD, InboundConfirmed,
            InboundCredential, InstantiateMsg, Network, OUTPUT_SIZE, OutboundConfirmed,
            OutboundRequested, SIGNATURE_SIZE, Transaction, Vout, create_tx_in,
        },
        gateway::{
            self, Remote,
            bridge::{BridgeMsg, TransferRemoteRequest},
        },
    },
    grug::{
        Addr, AuthCtx, AuthResponse, Coins, Hash256, HexByteArray, Inner, JsonDeExt, Message,
        MsgExecute, MutableCtx, Number, NumberConst, Order, QuerierExt as _, Response, StdResult,
        SudoCtx, Tx, Uint128,
    },
    std::{collections::BTreeMap, str::FromStr},
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> anyhow::Result<Response> {
    // Ensure the vault address is valid.
    check_bitcoin_address(&msg.config.vault, msg.config.network)?;

    // Ensure the vault address matches the one derived from the pub keys.
    ensure!(
        msg.config.vault == msg.config.multisig.address(msg.config.network).to_string(),
        "vault address must match the one derived from the multisig public keys;
         vault {}, derived {}",
        msg.config.vault,
        msg.config.multisig.address(msg.config.network),
    );

    CONFIG.save(ctx.storage, &msg.config)?;

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn authenticate(ctx: AuthCtx, tx: Tx) -> anyhow::Result<AuthResponse> {
    let mut msgs = tx.msgs.iter();

    // Assert the transaction contains exactly 1 MsgExecute.
    let (Some(Message::Execute(MsgExecute { contract, msg, .. })), None) =
        (msgs.next(), msgs.next())
    else {
        bail!("transaction must contain exactly one message");
    };

    // Assert the contract is the bridge.
    ensure!(
        contract == ctx.contract,
        "contract must be the bitcoin bridge"
    );

    let cfg = CONFIG.load(ctx.storage)?;

    if let Ok(ExecuteMsg::ObserveInbound(inbound_msg)) = msg.clone().deserialize_json() {
        let credential: InboundCredential = tx.credential.deserialize_json()?;

        ensure!(
            cfg.multisig.pub_keys().contains(&inbound_msg.pub_key),
            "public key `{}` is not a valid multisig public key",
            inbound_msg.pub_key.to_string()
        );

        // Verify the credential is valid.
        let secp = Secp256k1::verification_only();
        let msg = secp256k1::Message::from_digest_slice(&inbound_msg.hash()?)?;

        secp.verify_ecdsa(
            &msg,
            &Signature::from_der(credential.signature.inner())?,
            &PublicKey::from_slice(inbound_msg.pub_key.inner())?,
        )?;
    } else if let Ok(ExecuteMsg::AuthorizeOutbound {
        id,
        signatures,
        pub_key,
    }) = msg.clone().deserialize_json()
    {
        let tx = OUTBOUNDS.load(ctx.storage, id)?;

        ensure!(
            cfg.multisig.pub_keys().contains(&pub_key),
            "public key `{}` is not a valid multisig public key",
            pub_key.to_string()
        );

        ensure!(
            tx.inputs.len() == signatures.len(),
            "transaction `{id}` has {} inputs, but {} signatures were provided",
            tx.inputs.len(),
            signatures.len()
        );

        // Validate the signatures.
        let btc_transaction = tx.to_btc_transaction(cfg.network)?;
        for (i, ((hash, vout), amount)) in tx.inputs.iter().enumerate() {
            let signature = signatures.get(i).ok_or(anyhow!(
                "missing signature for input `{hash}:{vout}` of transaction `{id}`"
            ))?;

            let signature = Signature::from_der(&signature[..signature.len() - 1])?;

            // TODO: Can this be moved outside the loop?
            let mut cache = SighashCache::new(&btc_transaction);

            let sighash = cache.p2wsh_signature_hash(
                i,
                cfg.multisig.script(),
                Amount::from_sat(amount.into_inner() as u64),
                EcdsaSighashType::All,
            )?;

            let msg = secp256k1::Message::from_digest_slice(&sighash[..])?;

            let secp = Secp256k1::verification_only();
            secp.verify_ecdsa(&msg, &signature, &PublicKey::from_slice(pub_key.inner())?)?
        }
    } else {
        bail!("the execute message must be either `ObserveInbound` or `AuthorizeOutbound`");
    }

    Ok(AuthResponse::new().request_backrun(false))
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn execute(ctx: MutableCtx, msg: ExecuteMsg) -> anyhow::Result<Response> {
    match msg {
        ExecuteMsg::UpdateConfig {
            sats_per_vbyte,
            outbound_strategy,
        } => update_config(ctx, sats_per_vbyte, outbound_strategy),
        ExecuteMsg::ObserveInbound(inbound_msg) => observe_inbound(
            ctx,
            inbound_msg.transaction_hash,
            inbound_msg.vout,
            inbound_msg.amount,
            inbound_msg.recipient,
            inbound_msg.pub_key,
        ),
        ExecuteMsg::Bridge(BridgeMsg::TransferRemote { req, amount }) => {
            transfer_remote(ctx, req, amount)
        },
        ExecuteMsg::AuthorizeOutbound {
            id,
            signatures,
            pub_key,
        } => authorize_outbound(ctx, id, signatures, pub_key),
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
    pub_key: HexByteArray<33>,
) -> anyhow::Result<Response> {
    let cfg = CONFIG.load(ctx.storage)?;

    // Ensure only the bridge can call this function.
    ensure!(
        ctx.sender == ctx.contract,
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
        voters.insert(pub_key),
        "you've already voted for transaction `{hash}`"
    );

    // If a threshold number of votes has been reached:
    //
    // 1. Mint synthetic Bitcoin tokens to the recipient, if presents.
    // 2. Add the transaction to the available UTXO set.
    // 3. Add the UTXO to the processed UTXOs set (to prevent double spending).
    //
    // Otherwise, simply save the voters set, then we're done.
    // Note that, if the recipient is None, we cannot mint tokens, since
    // it could be the change of a withdrawal transaction.
    let (maybe_msg, maybe_event) = if voters.len() >= cfg.multisig.threshold() as usize {
        PROCESSED_UTXOS.insert(ctx.storage, (hash, vout))?;
        UTXOS.insert(ctx.storage, (amount, hash, vout))?;
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
            vout,
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
pub fn cron_execute(ctx: SudoCtx) -> anyhow::Result<Response> {
    let cfg = CONFIG.load(ctx.storage)?;

    // Take the pending outbound transfers from the storage.
    let outputs = OUTBOUND_QUEUE.drain(ctx.storage, None, None)?;

    // If there's no pending outbound transfers, nothing to do.
    if outputs.is_empty() {
        return Ok(Response::new());
    }

    // Each transaction can have a maximum number of outputs.
    let mut events = vec![];
    let mut iter = outputs.into_iter();

    // Keep creating transactions until there are no more outputs left.
    while iter.len() > 0 {
        let mut tx_output = BTreeMap::new();
        let mut withdraw_amount = Uint128::ZERO;

        for (k, v) in iter.by_ref() {
            tx_output.insert(k, v);
            withdraw_amount += v;

            // Check if the maximum number of outputs is reached for this tx.
            if tx_output.len() == cfg.max_output_per_tx {
                break;
            }
        }

        // Prepare the transaction in order to estimate the size and so the fee.
        let tx = Transaction {
            inputs: BTreeMap::new(),
            outputs: tx_output.clone(),
            fee: Uint128::ZERO,
        };

        let mut btc_transaction = tx.to_btc_transaction(cfg.network)?;

        // The fee is calculated as tx_size * sats_per_vbyte.
        // The function `vsize()` returns the size of the transaction in vbyte, but
        // it doesn't include the size of the signatures.
        let mut fee = Uint128::ZERO;

        // For each input, we need a number of signatures equal to the threshold.
        // So for each input, we calculate the size of the signatures in vbyte as
        // INPUT_SIGNATURES_OVERHEAD + SIGNATURE_SIZE * threshold.
        let signature_size_per_input = INPUT_SIGNATURES_OVERHEAD
            + SIGNATURE_SIZE * Uint128::new(cfg.multisig.threshold() as u128);

        // Choose the UTXOs as inputs for the outbound transaction.
        let mut inputs = BTreeMap::new();
        let mut inputs_amount = Uint128::ZERO;

        // Keep adding UTXOs until we reach the withdraw amount + fee.
        for res in UTXOS.range(ctx.storage, None, None, cfg.outbound_strategy) {
            if inputs_amount >= withdraw_amount + fee {
                break;
            }

            let (amount, hash, vout) = res?;

            inputs.insert((hash, vout), amount);
            inputs_amount += amount;

            // Add the input to the transaction for fee estimation.
            btc_transaction.input.push(create_tx_in(&hash, vout));

            // Size of signatures.
            let signatures_size = Uint128::new(inputs.len() as u128) * signature_size_per_input;

            // Adding 1 OUTPUT_SIZE to address the output size for the vault.
            fee = (Uint128::new(btc_transaction.vsize() as u128) + signatures_size + OUTPUT_SIZE)
                * cfg.sats_per_vbyte;
        }

        // Ensure we have enough UTXOs to cover the withdraw amount + fee.
        ensure!(
            inputs_amount >= withdraw_amount + fee,
            "not enough UTXOs to cover the withdraw amount + fee: {} < {}",
            inputs_amount,
            withdraw_amount + fee
        );

        // Total amount of BTC needed for this tx.
        let total = withdraw_amount + fee;

        // If there's excess input, send the excess back to the vault.
        if inputs_amount > total {
            tx_output.insert(cfg.vault.clone(), inputs_amount - total);
        }

        // Delete the chosen UTXOs.
        for ((hash, vout), amount) in &inputs {
            UTXOS.remove(ctx.storage, (*amount, *hash, *vout));
        }

        let (id, _) = OUTBOUND_ID.increment(ctx.storage)?;
        let transaction = Transaction {
            inputs,
            outputs: tx_output,
            fee,
        };

        // Save the outbound transaction.
        OUTBOUNDS.save(ctx.storage, id, &transaction)?;

        events.push(OutboundRequested { id, transaction });
    }

    Ok(Response::new().add_events(events)?)
}

fn authorize_outbound(
    ctx: MutableCtx,
    id: u32,
    signatures: Vec<BitcoinSignature>,
    pub_key: HexByteArray<33>,
) -> anyhow::Result<Response> {
    let cfg = CONFIG.load(ctx.storage)?;

    // Ensure only the bridge can call this function.
    ensure!(
        ctx.sender == ctx.contract,
        "you don't have the right, O you don't have the right"
    );

    // Add the signatures.
    let cumulative_signatures =
        SIGNATURES.may_update(ctx.storage, id, |cumulative_signatures| {
            let mut cumulative_signatures = cumulative_signatures.unwrap_or_default();

            if cumulative_signatures.len() >= cfg.multisig.threshold() as usize {
                bail!("transaction `{id}` already has enough signatures");
            }

            ensure!(
                cumulative_signatures.insert(pub_key, signatures).is_none(),
                "you've already signed transaction `{id}`"
            );

            Ok(cumulative_signatures)
        })?;

    Ok(Response::new().may_add_event(
        if cumulative_signatures.len() >= cfg.multisig.threshold() as usize {
            Some(OutboundConfirmed {
                id,
                transaction: OUTBOUNDS.load(ctx.storage, id)?,
                signatures: cumulative_signatures,
            })
        } else {
            None
        },
    )?)
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
