use {
    crate::{CONFIG, DELIVERIES, NONCE},
    anyhow::{anyhow, ensure},
    grug::{Addr, Coins, Hash, HexBinary, MutableCtx, QuerierExt, Response, StdResult},
    hyperlane_types::{
        hooks::{self, HookMsg, HookQuery, QueryHookRequest},
        isms::{IsmQuery, QueryIsmRequest},
        mailbox::{
            Dispatch, DispatchId, Domain, ExecuteMsg, InstantiateMsg, Message, Process, ProcessId,
            MAILBOX_VERSION,
        },
        recipients::{self, QueryRecipientRequest, RecipientMsg, RecipientQuery},
        Addr32,
    },
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> StdResult<Response> {
    CONFIG.save(ctx.storage, &msg.config)?;

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn execute(ctx: MutableCtx, msg: ExecuteMsg) -> anyhow::Result<Response> {
    match msg {
        ExecuteMsg::Dispatch {
            destination_domain,
            recipient,
            body,
            metadata,
            hook,
        } => dispatch(
            ctx,
            destination_domain,
            recipient,
            body,
            metadata.unwrap_or_default(),
            hook,
        ),
        ExecuteMsg::Process {
            raw_message,
            raw_metadata,
        } => process(ctx, raw_message, raw_metadata),
    }
}

#[inline]
fn dispatch(
    mut ctx: MutableCtx,
    destination_domain: Domain,
    recipient: Addr32,
    body: HexBinary,
    metadata: HexBinary,
    hook: Option<Addr>,
) -> anyhow::Result<Response> {
    let cfg = CONFIG.load(ctx.storage)?;
    let (nonce, _) = NONCE.increment(ctx.storage)?;

    // Ensure the destination domain is not the local domain.
    ensure!(
        destination_domain != cfg.local_domain,
        "Destination domain is the same as the local domain"
    );

    // Compose and encode the Hyperlane message.
    let message = Message {
        version: MAILBOX_VERSION,
        nonce,
        origin_domain: cfg.local_domain,
        sender: Addr32::from(ctx.sender),
        destination_domain,
        recipient,
        body,
    };

    let raw_message = message.encode();
    let message_id = Hash::from_inner(ctx.api.keccak256(&raw_message));

    // Query the required hook for fee amount.
    let fees = ctx
        .querier
        .query_wasm_smart(
            cfg.required_hook,
            QueryHookRequest(HookQuery::QuoteDispatch {
                raw_message: raw_message.clone(),
                raw_metadata: metadata.clone(),
            }),
        )?
        .as_quote_dispatch();

    // Deduct the fee from the received funds.
    // The fee will go to the required hook; the rest (if any) will go to the
    // sender specified hook, or the default hook if not specified.
    ctx.funds.deduct_many(fees.clone())?;

    Ok(Response::new()
        .add_message(grug::Message::execute(
            cfg.required_hook,
            &hooks::ExecuteMsg::Hook(HookMsg::PostDispatch {
                raw_message: raw_message.clone(),
                raw_metadata: metadata.clone(),
            }),
            fees,
        )?)
        .add_message(grug::Message::execute(
            hook.unwrap_or(cfg.default_hook),
            &hooks::ExecuteMsg::Hook(HookMsg::PostDispatch {
                raw_message,
                raw_metadata: metadata,
            }),
            ctx.funds,
        )?)
        .add_event(Dispatch(message))?
        .add_event(DispatchId { message_id })?)
}

#[inline]
fn process(
    ctx: MutableCtx,
    raw_message: HexBinary,
    raw_metadata: HexBinary,
) -> anyhow::Result<Response> {
    let cfg = CONFIG.load(ctx.storage)?;

    // Decode the Hyperlane message.
    let message = Message::decode(&raw_message)?;
    let message_id = Hash::from_inner(ctx.api.keccak256(&raw_message));
    let recipient = message.recipient.try_into()?;

    ensure!(
        message.version == MAILBOX_VERSION,
        "incorrect mailbox version! expecting: {MAILBOX_VERSION}, found: {}",
        message.version
    );

    ensure!(
        message.destination_domain == cfg.local_domain,
        "incorrect destination domain! expecting: {}, found: {}",
        cfg.local_domain,
        message.destination_domain
    );

    ensure!(
        !DELIVERIES.has(ctx.storage, message_id),
        "message has already been delivered! message id: {message_id}",
    );

    // Query the recipient's ISM.
    // If the recipient doesn't specify an ISM, use the default.
    let ism = ctx
        .querier
        .query_wasm_smart(
            recipient,
            QueryRecipientRequest(RecipientQuery::InterchainSecurityModule {}),
        )?
        .as_interchain_security_module()
        .unwrap_or(cfg.default_ism);

    // Query the ISM to verify the message.
    ctx.querier
        .query_wasm_smart(
            ism,
            QueryIsmRequest(IsmQuery::Verify {
                raw_message,
                raw_metadata,
            }),
        )
        .map(|res| res.as_verify())
        .map_err(|err| anyhow!("ISM verification failed: {err}"))?;

    // Mark the message as delivered.
    DELIVERIES.insert(ctx.storage, message_id)?;

    Ok(Response::new()
        .add_message(grug::Message::execute(
            recipient,
            &recipients::ExecuteMsg::Recipient(RecipientMsg::Handle {
                origin_domain: message.origin_domain,
                sender: message.sender,
                body: message.body,
            }),
            Coins::new(),
        )?)
        .add_event(Process {
            origin_domain: message.origin_domain,
            sender: message.sender,
            recipient: message.recipient,
        })?
        .add_event(ProcessId { message_id })?)
}
