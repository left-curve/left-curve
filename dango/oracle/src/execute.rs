use {
    crate::{state::GUARDIAN_SETS, PRICES, PRICE_SOURCES},
    anyhow::{bail, ensure},
    dango_types::oracle::{ExecuteMsg, InstantiateMsg, PriceSource, PythId, PythVaa},
    grug::{
        AuthCtx, AuthMode, AuthResponse, Binary, Denom, Inner, JsonDeExt, Message, MsgExecute,
        MutableCtx, QuerierExt, Response, Tx,
    },
    std::collections::BTreeMap,
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> anyhow::Result<Response> {
    for (i, guardian_set) in msg.guardian_sets {
        GUARDIAN_SETS.save(ctx.storage, i, &guardian_set)?;
    }

    for (denom, price_source) in msg.price_sources {
        PRICE_SOURCES.save(ctx.storage, &denom, &price_source)?;
    }

    Ok(Response::new())
}

/// The oracle can be used as sender when:
///
/// - Auth mode must be `Finalize`. This ensures such transactions are only
///   inserted by the block proposer during ABCI++ `PrepareProposal`, not by
///   regular users.
/// - The transaction contains exactly one message.
/// - This one message is an `Execute`.
/// - The contract being executed must be the oracle itself.
/// - the execute message must be `FeedPrices`.
#[cfg_attr(not(feature = "library"), grug::export)]
pub fn authenticate(ctx: AuthCtx, tx: Tx) -> anyhow::Result<AuthResponse> {
    // Authenticate can only be called during finalize.
    ensure!(
        ctx.mode == AuthMode::Finalize,
        "you don't have the right, O you don't have the right"
    );

    let mut msgs = tx.msgs.iter();

    // Assert the transaction contains exactly 1 MsgExecute.
    let (Some(Message::Execute(MsgExecute { contract, msg, .. })), None) =
        (msgs.next(), msgs.next())
    else {
        bail!("transaction must contain exactly one message");
    };

    // Assert the contract is the oracle.
    ensure!(contract == ctx.contract, "contract must be the oracle");

    // Assert the message is `ExecuteMsg::FeedPrices`.
    let Ok(ExecuteMsg::FeedPrices(..)) = msg.clone().deserialize_json() else {
        bail!("the execute message must be feed prices");
    };

    Ok(AuthResponse::new().request_backrun(false))
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn execute(ctx: MutableCtx, msg: ExecuteMsg) -> anyhow::Result<Response> {
    match msg {
        ExecuteMsg::RegisterPriceSources(price_sources) => {
            register_price_sources(ctx, price_sources)
        },
        ExecuteMsg::FeedPrices(vaas) => feed_prices(ctx, vaas.into_inner()),
    }
}

fn register_price_sources(
    ctx: MutableCtx,
    price_sources: BTreeMap<Denom, PriceSource>,
) -> anyhow::Result<Response> {
    // Only chain owner can register a denom.
    ensure!(
        ctx.sender == ctx.querier.query_owner()?,
        "you don't have the right, O you don't have the right"
    );

    for (denom, price_source) in price_sources {
        PRICE_SOURCES.save(ctx.storage, &denom, &price_source)?;
    }

    Ok(Response::new())
}

fn feed_prices(ctx: MutableCtx, vaas: Vec<Binary>) -> anyhow::Result<Response> {
    // Only the oracle can feed prices.
    // ensure!(
    //     ctx.sender == ctx.contract,
    //     "you don't have the right, O you don't have the right"
    // );

    let compare: fn(&u64, &u64) -> bool = if ctx.sender == ctx.contract {
        u64::ge
    } else {
        u64::gt
    };

    for vaa in vaas {
        // Deserialize the Pyth VAA from binary.
        let vaa = PythVaa::new(ctx.api, vaa.into_inner())?;

        // Verify the VAA, and store the prices.
        for feed in vaa.verify(ctx.storage, ctx.api, ctx.block, GUARDIAN_SETS)? {
            let hash = PythId::from_inner(feed.id.to_bytes());

            // Save the price if there isn't already a price saved, or if the
            // new price is not older. This is due to the fact that Pyth return
            // the publish time with a precision of seconds but the SSE connection
            // returns more events per seconds.
            PRICES.may_update(ctx.storage, hash, |maybe_price| -> anyhow::Result<_> {
                if let Some(price) = maybe_price {
                    // if feed.get_price_unchecked().publish_time as u64 >= price.timestamp {
                    if compare(
                        &(feed.get_price_unchecked().publish_time as u64),
                        &price.timestamp,
                    ) {
                        feed.try_into()
                    } else {
                        Ok(price)
                    }
                } else {
                    feed.try_into()
                }
            })?;
        }
    }

    Ok(Response::new())
}
