use {
    crate::{CONFIG, DELIVERIES, LATEST_DISPATCHED_ID, MAILBOX_VERSION, NONCE},
    anyhow::ensure,
    grug::{Addr, Coins, HexBinary, HexByteArray, MutableCtx, Response, StdResult},
    hyperlane_types::{
        hook::{self, QueryQuoteDispatchRequest},
        ism::QueryVerifyRequest,
        mailbox::{Dispatch, DispatchId, ExecuteMsg, InstantiateMsg, Message, Process, ProcessId},
        recipient::{self, QueryInterchainSecurityModuleRequest},
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
            metadata,
        } => process(ctx, raw_message, metadata),
    }
}

#[inline]
fn dispatch(
    mut ctx: MutableCtx,
    destination_domain: u32,
    recipient: Addr32,
    body: HexBinary,
    metadata: HexBinary,
    hook: Option<Addr>,
) -> anyhow::Result<Response> {
    let cfg = CONFIG.load(ctx.storage)?;
    let (nonce, _) = NONCE.increment(ctx.storage)?;

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
    let message_id = HexByteArray::from_inner(ctx.api.keccak256(&raw_message));

    // Query the required hook for fee amount.
    let fees = ctx
        .querier
        .query_wasm_smart(cfg.required_hook, QueryQuoteDispatchRequest {
            raw_message: raw_message.clone(),
            metadata: metadata.clone(),
        })?;

    // Deduct the fee from the received funds.
    // The fee will go to the required hook; the rest (if any) will go to the
    // sender specified hook, or the default hook if not specified.
    ctx.funds.deduct_many(fees.clone())?;

    // Commit the message.
    LATEST_DISPATCHED_ID.save(ctx.storage, &message_id)?;

    Ok(Response::new()
        .add_message(grug::Message::execute(
            cfg.required_hook,
            &hook::ExecuteMsg::PostDispatch {
                raw_message: raw_message.clone(),
                metadata: metadata.clone(),
            },
            fees,
        )?)
        .add_message(grug::Message::execute(
            hook.unwrap_or(cfg.default_hook),
            &hook::ExecuteMsg::PostDispatch {
                raw_message,
                metadata,
            },
            ctx.funds,
        )?)
        .add_event("mailbox_dispatch", &Dispatch {
            sender: message.sender,
            destination: message.destination_domain,
            recipient: message.recipient,
            message: message.body,
        })?
        .add_event("mailbox_dispatch_id", &DispatchId { message_id })?)
}

#[inline]
fn process(
    ctx: MutableCtx,
    raw_message: HexBinary,
    metadata: HexBinary,
) -> anyhow::Result<Response> {
    let cfg = CONFIG.load(ctx.storage)?;

    // Decode the Hyperlane message.
    let message = Message::decode(&raw_message);
    let message_id = HexByteArray::from_inner(ctx.api.keccak256(&raw_message));
    let recipient = message.recipient.try_into()?;

    ensure!(
        message.version == MAILBOX_VERSION,
        "incorrect mailbox version! expecting: {}, found: {}",
        MAILBOX_VERSION,
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
        "message has already been delivered! message id: {}",
        message_id
    );

    // Query the recipient's ISM.
    // If the recipient doesn't specify an ISM, use the default.
    let ism = ctx
        .querier
        .query_wasm_smart(recipient, QueryInterchainSecurityModuleRequest {})?
        .unwrap_or(cfg.default_ism);

    // Query the ISM to verify the message.
    ctx.querier.query_wasm_smart(ism, QueryVerifyRequest {
        raw_message,
        metadata,
    })?;

    // Commit the delivery.
    DELIVERIES.insert(ctx.storage, message_id)?;

    Ok(Response::new()
        .add_message(grug::Message::execute(
            recipient,
            &recipient::ExecuteMsg::Handle {
                origin: message.origin_domain,
                sender: message.sender,
                body: message.body,
            },
            Coins::new(),
        )?)
        .add_event("mailbox_process", &Process {
            origin: message.origin_domain,
            sender: message.sender,
            recipient: message.recipient,
        })?
        .add_event("mailbox_process_id", &ProcessId { message_id })?)
}
