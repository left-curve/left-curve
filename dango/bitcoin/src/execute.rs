use {
    crate::{
        CONFIG, INBOUNDS, OUTBOUND_ID, OUTBOUND_QUEUE, OUTBOUNDS, PROCESSED_UTXOS, SIGNATURES,
        UTXOS,
    },
    anyhow::{bail, ensure},
    corepc_client::bitcoin::{
        Address, Amount, EcdsaSighashType, Transaction as BtcTransaction,
        key::Secp256k1,
        secp256k1::{self, PublicKey, ecdsa::Signature},
        sighash::SighashCache,
    },
    dango_types::{
        DangoQuerier,
        bitcoin::{
            BitcoinSignature, Config, ExecuteMsg, INPUT_SIGNATURES_OVERHEAD, InboundConfirmed,
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
        MsgExecute, MutableCtx, Number, NumberConst, Order, PrefixBound, QuerierExt as _, Response,
        StdResult, Storage, SudoCtx, Tx, Uint128,
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

    let (Some(Message::Execute(MsgExecute { contract, msg, .. })), None) =
        (msgs.next(), msgs.next())
    else {
        bail!("transaction must contain exactly one message");
    };

    ensure!(
        contract == ctx.contract,
        "contract must be the bitcoin bridge"
    );

    let cfg = CONFIG.load(ctx.storage)?;

    // The only allowed messages are `ObserveInbound` and `AuthorizeOutbound`.
    match msg.clone().deserialize_json() {
        Ok(ExecuteMsg::ObserveInbound(inbound_msg)) => {
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
        },

        Ok(ExecuteMsg::AuthorizeOutbound {
            id,
            signatures,
            pub_key,
        }) => {
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
            let mut cache = SighashCache::new(tx.to_btc_transaction(cfg.network)?);

            for (i, (_, amount)) in tx.inputs.iter().enumerate() {
                let signature = signatures.get(i).unwrap();
                // Remove the last byte, which is the sighash type.
                let signature = Signature::from_der(&signature[..signature.len() - 1])?;

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
        },

        _ => bail!("the execute message must be either `ObserveInbound` or `AuthorizeOutbound`"),
    };

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

        // Choose the best UTXOs for this transaction.
        let withdraw_helper = select_best_utxos(ctx.storage, cfg.clone(), tx_output.clone())?;

        // If there's excess input, send the excess back to the vault.
        let surplus_amount = withdraw_helper.surplus_amount();
        if surplus_amount > Uint128::ZERO {
            tx_output.insert(cfg.vault.clone(), surplus_amount);
        }

        // Delete the chosen UTXOs.
        for ((hash, vout), amount) in &withdraw_helper.inputs {
            UTXOS.remove(ctx.storage, (*amount, *hash, *vout));
        }

        let (id, _) = OUTBOUND_ID.increment(ctx.storage)?;
        let fee = withdraw_helper.fee();
        let transaction = Transaction {
            inputs: withdraw_helper.inputs,
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

/// Select the best UTXOs to cover the withdraw amount and fee.
fn select_best_utxos(
    storage: &mut dyn Storage,
    cfg: Config,
    outputs: BTreeMap<String, Uint128>,
) -> anyhow::Result<WithdrawHelper> {
    let mut withdraw_helper = WithdrawHelper::new(outputs, &cfg)?;

    while withdraw_helper.remaining_amount() > Uint128::ZERO {
        let remaining_amount = withdraw_helper.remaining_amount();

        let maybe_bigger = {
            let mut value = None;

            for res in UTXOS.prefix_range(
                storage,
                Some(PrefixBound::Inclusive(remaining_amount)),
                None,
                Order::Ascending,
            ) {
                // Ensure the input is not already used.
                let (amount, hash, vout) = res?;
                if !withdraw_helper.input_already_used(hash, vout) {
                    value = Some((amount, hash, vout));
                    break;
                }
            }

            value
        };

        let maybe_lower = {
            let mut value = None;
            for res in UTXOS.prefix_range(
                storage,
                None,
                Some(PrefixBound::Exclusive(remaining_amount)),
                Order::Descending,
            ) {
                // Ensure the input is not already used.
                let (amount, hash, vout) = res?;
                if !withdraw_helper.input_already_used(hash, vout) {
                    value = Some((amount, hash, vout));
                    break;
                }
            }

            value
        };

        // Select the UTXO with the lowest delta.
        let (amount, hash, vout) = {
            // If the remaining amount is less than 10_000 sats, we select the bigger one.
            if let (Some(bigger), true) = (maybe_bigger, remaining_amount < Uint128::new(10_000)) {
                bigger
            } else {
                match (maybe_bigger, maybe_lower) {
                    (Some(bigger), Some(lower)) => {
                        if bigger.0 - remaining_amount < remaining_amount - lower.0 {
                            bigger
                        } else {
                            lower
                        }
                    },
                    (Some(bigger), None) => bigger,
                    (None, Some(lower)) => lower,
                    (None, None) => bail!(
                        "not enough UTXOs to cover the withdraw amount + fee: {} < {}",
                        withdraw_helper.inputs_amount,
                        withdraw_helper.withdraw_amount + withdraw_helper.fee()
                    ),
                }
            }
        };

        // Add the selected input to the transaction.
        withdraw_helper.add_input(hash, vout, amount)?;
    }
    Ok(withdraw_helper)
}

struct WithdrawHelper {
    tx: BtcTransaction,
    signature_size_per_input: Uint128,
    inputs: BTreeMap<(Hash256, Vout), Uint128>,
    inputs_amount: Uint128,
    sats_per_vbyte: Uint128,
    withdraw_amount: Uint128,
}
impl WithdrawHelper {
    pub fn new(outputs: BTreeMap<String, Uint128>, config: &Config) -> anyhow::Result<Self> {
        let mut withdraw_amount = Uint128::ZERO;
        for amount in outputs.values() {
            withdraw_amount += *amount;
        }

        let tx = Transaction {
            inputs: BTreeMap::new(),
            outputs,
            fee: Uint128::ZERO,
        }
        .to_btc_transaction(config.network)?;

        let signature_size_per_input = INPUT_SIGNATURES_OVERHEAD
            + SIGNATURE_SIZE * Uint128::new(config.multisig.threshold() as u128);

        Ok(Self {
            tx,
            signature_size_per_input,
            inputs: BTreeMap::new(),
            inputs_amount: Uint128::ZERO,
            sats_per_vbyte: config.sats_per_vbyte,
            withdraw_amount,
        })
    }

    pub fn fee(&self) -> Uint128 {
        let signatures_size =
            Uint128::new(self.inputs.len() as u128) * self.signature_size_per_input;

        (Uint128::new(self.tx.vsize() as u128) + signatures_size + OUTPUT_SIZE)
            * self.sats_per_vbyte
    }

    pub fn add_input(&mut self, hash: Hash256, vout: Vout, amount: Uint128) -> anyhow::Result<()> {
        if self.input_already_used(hash, vout) {
            bail!("input `{hash}:{vout}` already exists in the transaction");
        }

        self.inputs.insert((hash, vout), amount);
        self.tx.input.push(create_tx_in(&hash, vout));
        self.inputs_amount += amount;

        Ok(())
    }

    /// Return true if the input exists in the transaction.
    pub fn input_already_used(&self, hash: Hash256, vout: Vout) -> bool {
        self.inputs.contains_key(&(hash, vout))
    }

    /// Return the remaining amount that needs to cover the withdraw_amount + fee.
    pub fn remaining_amount(&self) -> Uint128 {
        let fee = self.fee();

        if self.inputs_amount >= self.withdraw_amount + fee {
            Uint128::ZERO
        } else {
            self.withdraw_amount + fee - self.inputs_amount
        }
    }

    /// Return the surplus amount that can be sent back to the vault.
    pub fn surplus_amount(&self) -> Uint128 {
        if self.inputs_amount > self.withdraw_amount + self.fee() {
            self.inputs_amount - (self.withdraw_amount + self.fee())
        } else {
            Uint128::ZERO
        }
    }
}
