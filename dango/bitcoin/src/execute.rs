use {
    crate::{
        ADDRESS_INDEX, ADDRESSES, CONFIG, INBOUNDS, OUTBOUND_ID, OUTBOUND_QUEUE, OUTBOUNDS,
        PROCESSED_UTXOS, SIGNATURES, UTXOS,
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
            InboundCredential, InboundMsg, InstantiateMsg, MultisigWallet, Network, OUTPUT_SIZE,
            OutboundConfirmed, OutboundRequested, Recipient, SIGNATURE_SIZE, Transaction, Vout,
            create_tx_in,
        },
        gateway::{
            self, Remote,
            bridge::{BridgeMsg, TransferRemoteRequest},
        },
    },
    grug::{
        Addr, AuthCtx, AuthResponse, Coins, Hash256, HexByteArray, Inner, JsonDeExt, Message,
        MsgExecute, MutableCtx, Number, NumberConst, Order, PrefixBound, QuerierExt, Response,
        StdResult, Storage, SudoCtx, Tx, Uint128,
    },
    std::{collections::BTreeMap, str::FromStr},
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> anyhow::Result<Response> {
    // Ensure the vault address is valid.
    check_bitcoin_address(&msg.config.vault, msg.config.network)?;

    // Ensure the vault address matches the one derived from the pub keys.
    let multisig_wallet = MultisigWallet::new(&msg.config.multisig, &Recipient::Vault);
    ensure!(
        msg.config.vault == multisig_wallet.address(msg.config.network).to_string(),
        "vault address must match the one derived from the multisig public keys;
         vault {}, derived {}",
        msg.config.vault,
        multisig_wallet.address(msg.config.network),
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
                cfg.multisig
                    .pub_keys_as_bytes_array()
                    .contains(&inbound_msg.pub_key),
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

            // Ensure this message has not already been delivered.
            check_inbound(ctx.storage, cfg, inbound_msg)?;
        },

        Ok(ExecuteMsg::AuthorizeOutbound {
            id,
            signatures,
            pub_key,
        }) => {
            let tx = OUTBOUNDS.load(ctx.storage, id)?;

            ensure!(
                cfg.multisig.pub_keys_as_bytes_array().contains(&pub_key),
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

            for (i, (_, (amount, recipient))) in tx.inputs.iter().enumerate() {
                let signature = signatures.get(i).unwrap();
                // Remove the last byte, which is the sighash type.
                let signature = Signature::from_der(&signature[..signature.len() - 1])?;

                let multisig = MultisigWallet::new(&cfg.multisig, recipient);

                let sighash = cache.p2wsh_signature_hash(
                    i,
                    multisig.script(),
                    Amount::from_sat(amount.into_inner() as u64),
                    EcdsaSighashType::All,
                )?;

                let msg = secp256k1::Message::from_digest_slice(&sighash[..])?;

                let secp = Secp256k1::verification_only();
                secp.verify_ecdsa(&msg, &signature, &PublicKey::from_slice(pub_key.inner())?)?
            }

            // Ensure this message has not already been delivered.
            check_outbound(ctx.storage, cfg, id, pub_key)?;
        },

        _ => bail!("the execute message must be either `ObserveInbound` or `AuthorizeOutbound`"),
    };

    Ok(AuthResponse::new().request_backrun(false))
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn execute(ctx: MutableCtx, msg: ExecuteMsg) -> anyhow::Result<Response> {
    match msg {
        ExecuteMsg::UpdateConfig {
            fee_rate_updater,
            minimum_deposit,
            max_output_per_tx,
        } => update_config(ctx, fee_rate_updater, minimum_deposit, max_output_per_tx),
        ExecuteMsg::UpdateFeeRate(sats_per_vbyte) => update_fee_rate(ctx, sats_per_vbyte),
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
        ExecuteMsg::CreateDepositAddress {} => create_deposit_address(ctx),
    }
}

fn update_config(
    ctx: MutableCtx,
    fee_rate_updater: Option<Addr>,
    minimum_deposit: Option<Uint128>,
    max_output_per_tx: Option<usize>,
) -> anyhow::Result<Response> {
    ensure!(
        ctx.sender == ctx.querier.query_owner()?,
        "you don't have the right, O you don't have the right"
    );

    let mut config = CONFIG.load(ctx.storage)?;

    if let Some(fee_rate_updater) = fee_rate_updater {
        config.fee_rate_updater = fee_rate_updater;
    }

    if let Some(minimum_deposit) = minimum_deposit {
        config.minimum_deposit = minimum_deposit;
    }

    if let Some(max_output_per_tx) = max_output_per_tx {
        ensure!(
            max_output_per_tx > 0,
            "max_output_per_tx must be greater than 0"
        );
        config.max_output_per_tx = max_output_per_tx;
    }

    CONFIG.save(ctx.storage, &config)?;

    Ok(Response::new())
}

fn update_fee_rate(ctx: MutableCtx, sats_per_vbyte: Uint128) -> anyhow::Result<Response> {
    let mut config = CONFIG.load(ctx.storage)?;

    ensure!(
        ctx.sender == config.fee_rate_updater || ctx.sender == ctx.querier.query_owner()?,
        "you don't have the right, O you don't have the right"
    );

    config.sats_per_vbyte = sats_per_vbyte;

    CONFIG.save(ctx.storage, &config)?;

    Ok(Response::new())
}

fn observe_inbound(
    ctx: MutableCtx,
    hash: Hash256,
    vout: Vout,
    amount: Uint128,
    recipient: Recipient,
    pub_key: HexByteArray<33>,
) -> anyhow::Result<Response> {
    let cfg = CONFIG.load(ctx.storage)?;

    // Ensure only the bitcoin bridge can call this function.
    ensure!(
        ctx.sender == ctx.contract,
        "you don't have the right, O you don't have the right"
    );

    let inbound = (hash, vout, amount, recipient.clone());
    let mut voters = INBOUNDS
        .may_load(ctx.storage, inbound.clone())?
        .unwrap_or_default();

    ensure!(
        voters.insert(pub_key),
        "you've already voted for transaction `{hash}`"
    );

    // Check if the threshold has been reached.
    let (maybe_msg, maybe_event) = if voters.len() < cfg.multisig.threshold() as usize {
        // The threshold has not been reached yet, just save the voters set.
        INBOUNDS.save(ctx.storage, inbound, &voters)?;

        (None, None)
    } else {
        // The threshold has been reached:
        //
        // 1. Mint Bitcoin tokens to the recipient, if it's a user.
        // 2. Add the transaction to the available UTXO set.
        // 3. Add the UTXO to the processed UTXOs set (to prevent double spending).
        //
        // Note that, if the recipient is Vault, we cannot mint tokens, since
        // it's the change of a withdrawal transaction.
        PROCESSED_UTXOS.insert(ctx.storage, (hash, vout))?;
        UTXOS.save(ctx.storage, (amount, hash, vout), &recipient)?;
        INBOUNDS.remove(ctx.storage, inbound);

        // If there's an address index, mint the Bitcoin tokens to the user.
        let gateway = ctx.querier.query_gateway()?;

        let maybe_msg = match recipient {
            Recipient::Vault => None,
            Recipient::Index(index) => {
                // Load the deposit address related to the address index.
                let (addr, _) = ADDRESSES.idx.address_index.load(ctx.storage, index)?;

                Some(Message::execute(
                    gateway,
                    &gateway::ExecuteMsg::ReceiveRemote {
                        remote: Remote::Bitcoin,
                        amount,
                        recipient: addr,
                    },
                    Coins::new(),
                )?)
            },
            Recipient::Address(addr) => Some(Message::execute(
                gateway,
                &gateway::ExecuteMsg::ReceiveRemote {
                    remote: Remote::Bitcoin,
                    amount,
                    recipient: addr,
                },
                Coins::new(),
            )?),
        };

        let event = InboundConfirmed {
            transaction_hash: hash,
            vout,
            amount,
            recipient,
        };

        (maybe_msg, Some(event))
    };

    Ok(Response::new()
        .may_add_message(maybe_msg)
        .may_add_event(maybe_event)?)
}

fn authorize_outbound(
    ctx: MutableCtx,
    id: u32,
    signatures: Vec<BitcoinSignature>,
    pub_key: HexByteArray<33>,
) -> anyhow::Result<Response> {
    let cfg = CONFIG.load(ctx.storage)?;

    // Ensure only the bitcoin bridge can call this function.
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

fn transfer_remote(
    ctx: MutableCtx,
    req: TransferRemoteRequest,
    amount: Uint128,
) -> anyhow::Result<Response> {
    // Ensure only the gateway can call this function, that is responsible also
    // to deduct the withdrawal fees. The amount here is the net amount to withdraw.
    ensure!(
        ctx.sender == ctx.querier.query_gateway()?,
        "only gateway can call `transfer_remote`"
    );

    let TransferRemoteRequest::Bitcoin { recipient } = req else {
        bail!("incorrect TransferRemoteRequest type! expected: Bitcoin, found: {req:?}");
    };

    let cfg = CONFIG.load(ctx.storage)?;

    // Ensure the withdrawal amount is greater than min withdrawal.
    ensure!(
        amount >= cfg.min_withdrawal,
        "minimum withdrawal not met: {} < {}",
        amount,
        cfg.min_withdrawal
    );

    // Ensure the recipient address is valid.
    check_bitcoin_address(&recipient, cfg.network)?;

    // TODO: remove this check?
    ensure!(
        recipient != cfg.vault,
        "cannot withdraw to the vault address"
    );

    // If there is already a withdrawal to the same recipient, accumulate the amount.
    OUTBOUND_QUEUE.may_update(ctx.storage, recipient, |outbound| -> StdResult<_> {
        Ok(outbound.unwrap_or_default().checked_add(amount)?)
    })?;

    Ok(Response::new())
}

fn create_deposit_address(ctx: MutableCtx) -> anyhow::Result<Response> {
    let (index, _) = ADDRESS_INDEX.increment(ctx.storage)?;

    // Add a deposit address for the user if he hasn't already one.
    ADDRESSES.may_update(
        ctx.storage,
        ctx.sender,
        |maybe_address| match maybe_address {
            Some(_) => bail!("you already have a deposit address"),
            None => Ok(index),
        },
    )?;

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

        for (k, v) in iter.by_ref() {
            tx_output.insert(k, v);

            // Check if the maximum number of outputs is reached for this tx.
            if tx_output.len() == cfg.max_output_per_tx {
                break;
            }
        }

        // Choose the best UTXOs for this transaction.
        let transaction_builder = select_best_utxos(ctx.storage, cfg.clone(), tx_output.clone())?;

        // If there's excess input, send the excess back to the vault.
        let surplus_amount = transaction_builder.surplus_amount();
        // TODO: should we check that the surplus amount is above the minimum deposit?
        // If so, the fees will be higher and the tx could be faster.
        if surplus_amount > Uint128::ZERO {
            tx_output.insert(cfg.vault.clone(), surplus_amount);
        }

        // Delete the chosen UTXOs.
        for ((hash, vout), (amount, _)) in &transaction_builder.inputs {
            UTXOS.remove(ctx.storage, (*amount, *hash, *vout));
        }

        let (id, _) = OUTBOUND_ID.increment(ctx.storage)?;
        let fee = transaction_builder.fee();
        let transaction = Transaction {
            inputs: transaction_builder.inputs,
            outputs: tx_output,
            fee,
        };

        // Save the outbound transaction.
        OUTBOUNDS.save(ctx.storage, id, &transaction)?;

        events.push(OutboundRequested { id, transaction });
    }

    Ok(Response::new().add_events(events)?)
}

/// Ensure the inbound message is valid and has not already been processed.
fn check_inbound(
    storage: &mut dyn Storage,
    cfg: Config,
    inbound_msg: InboundMsg,
) -> anyhow::Result<()> {
    // Ensure the amount meets the minimum deposit requirement.
    ensure!(
        inbound_msg.amount >= cfg.minimum_deposit,
        "minimum deposit not met: {} < {}",
        inbound_msg.amount,
        cfg.minimum_deposit
    );

    // Ensure the UTXO has not been processed yet.
    ensure!(
        !PROCESSED_UTXOS.has(storage, (inbound_msg.transaction_hash, inbound_msg.vout)),
        "transaction `{} - {}` already exists in UTXO set",
        inbound_msg.transaction_hash,
        inbound_msg.vout
    );

    // Load the current voters for this inbound message.
    let inbound = (
        inbound_msg.transaction_hash,
        inbound_msg.vout,
        inbound_msg.amount,
        inbound_msg.recipient,
    );

    let voters = INBOUNDS
        .may_load(storage, inbound.clone())?
        .unwrap_or_default();

    // Ensure that the sender has not already voted for this inbound message.
    ensure!(
        !voters.contains(&inbound_msg.pub_key),
        "you've already voted for transaction `{} - {}`",
        inbound_msg.transaction_hash,
        inbound_msg.vout
    );

    Ok(())
}

/// Ensure the outbound message is valid and has not already been processed.
fn check_outbound(
    storage: &mut dyn Storage,
    cfg: Config,
    id: u32,
    pub_key: HexByteArray<33>,
) -> anyhow::Result<()> {
    // Ensure the transaction exists.
    OUTBOUNDS.load(storage, id)?;

    match SIGNATURES.load(storage, id) {
        Ok(cumulative_signatures) => {
            // Ensure that the transaction has not already enough signatures.
            ensure!(
                cumulative_signatures.len() < cfg.multisig.threshold() as usize,
                "transaction `{id}` already has enough signatures"
            );

            // Ensure that the sender has not already signed this outbound message.
            ensure!(
                !cumulative_signatures.contains_key(&pub_key),
                "you've already signed transaction `{id}`"
            );
        },
        Err(_) => {
            // There are no signatures yet, do nothing.
        },
    };

    Ok(())
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
) -> anyhow::Result<TransactionBuilder> {
    let mut transaction_builder = TransactionBuilder::new(outputs, &cfg)?;

    while transaction_builder.remaining_amount() > Uint128::ZERO {
        let remaining_amount = transaction_builder.remaining_amount();

        // Find the closest UTXO greater than or equal to the remaining amount.
        let maybe_bigger = {
            let mut value = None;

            for res in UTXOS.prefix_range(
                storage,
                Some(PrefixBound::Inclusive(remaining_amount)),
                None,
                Order::Ascending,
            ) {
                // Ensure the input is not already used.
                let ((amount, hash, vout), recipient_index) = res?;
                if !transaction_builder.input_already_used(hash, vout) {
                    value = Some((amount, hash, vout, recipient_index));
                    break;
                }
            }

            value
        };

        // Find the closest UTXO less than the remaining amount.
        let maybe_lower = {
            let mut value = None;
            for res in UTXOS.prefix_range(
                storage,
                None,
                Some(PrefixBound::Exclusive(remaining_amount)),
                Order::Descending,
            ) {
                // Ensure the input is not already used.
                let ((amount, hash, vout), recipient_index) = res?;
                if !transaction_builder.input_already_used(hash, vout) {
                    value = Some((amount, hash, vout, recipient_index));
                    break;
                }
            }

            value
        };

        // Select the UTXO with the lowest delta.
        let (amount, hash, vout, recipient) = {
            // If the remaining amount is less than 10_000 sats, we select the bigger one, in order
            // to avoid to iterating too much in the UTXO set.
            if let (Some(bigger), true) = (&maybe_bigger, remaining_amount < Uint128::new(10_000)) {
                bigger.clone()
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
                        transaction_builder.inputs_amount,
                        transaction_builder.withdraw_amount + transaction_builder.fee()
                    ),
                }
            }
        };

        // Add the selected input to the transaction.
        transaction_builder.add_input(hash, vout, amount, recipient)?;
    }
    Ok(transaction_builder)
}

/// Helper struct used to build a transaction and calculate fees.
struct TransactionBuilder {
    tx: BtcTransaction,
    signature_size_per_input: Uint128,
    inputs: BTreeMap<(Hash256, Vout), (Uint128, Recipient)>,
    inputs_amount: Uint128,
    sats_per_vbyte: Uint128,
    withdraw_amount: Uint128,
}

impl TransactionBuilder {
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

    pub fn add_input(
        &mut self,
        hash: Hash256,
        vout: Vout,
        amount: Uint128,
        recipient: Recipient,
    ) -> anyhow::Result<()> {
        if self.input_already_used(hash, vout) {
            bail!("input `{hash}:{vout}` already exists in the transaction");
        }

        self.inputs.insert((hash, vout), (amount, recipient));
        self.tx.input.push(create_tx_in(&hash, vout));
        self.inputs_amount += amount;

        Ok(())
    }

    /// Return true if the input exists in the transaction.
    pub fn input_already_used(&self, hash: Hash256, vout: Vout) -> bool {
        self.inputs.contains_key(&(hash, vout))
    }

    /// Calculate the fee for the current transaction in satoshis.
    /// At this point, the transaction does not include signatures yet.
    /// To calculate the fee, we consider the size of the transaction +
    /// the size of the signatures + 1 output (for change back to the vault).
    pub fn fee(&self) -> Uint128 {
        let signatures_size =
            Uint128::new(self.inputs.len() as u128) * self.signature_size_per_input;

        (Uint128::new(self.tx.vsize() as u128) + signatures_size + OUTPUT_SIZE)
            * self.sats_per_vbyte
    }

    /// Return the total amount needed to cover the withdraw_amount + fee.
    pub fn amounts_needed(&self) -> Uint128 {
        self.withdraw_amount + self.fee()
    }

    /// Return the remaining amount that needs to cover the withdraw_amount + fee.
    pub fn remaining_amount(&self) -> Uint128 {
        let amount_needed = self.amounts_needed();

        if self.inputs_amount >= amount_needed {
            Uint128::ZERO
        } else {
            amount_needed - self.inputs_amount
        }
    }

    /// Return the surplus amount that can be sent back to the vault.
    pub fn surplus_amount(&self) -> Uint128 {
        let amount_needed = self.amounts_needed();

        if self.inputs_amount > amount_needed {
            self.inputs_amount - amount_needed
        } else {
            Uint128::ZERO
        }
    }
}
