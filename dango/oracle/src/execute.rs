use {
    crate::{state::GUARDIAN_SETS, PRICES, PRICE_SOURCES},
    anyhow::{bail, ensure},
    dango_types::oracle::{
        ExecuteMsg, InstantiateMsg, PrecisionlessPrice, PriceSource, PythId, PythVaa,
    },
    grug::{
        AuthCtx, AuthMode, AuthResponse, Binary, Denom, Inner, JsonDeExt, Message, MsgExecute,
        MutableCtx, QuerierExt, Response, Tx,
    },
    std::{cmp::Ordering, collections::BTreeMap},
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
    for vaa in vaas {
        // Deserialize the Pyth VAA from binary.
        let vaa = PythVaa::new(ctx.api, vaa.into_inner())?;
        let new_sequence = vaa.wormhole_vaa.sequence;

        // Verify the VAA, and store the prices.
        for feed in vaa.verify(ctx.storage, ctx.api, ctx.block, GUARDIAN_SETS)? {
            let id = PythId::from_inner(feed.id.to_bytes());
            let new_price = PrecisionlessPrice::try_from(feed)?;

            // Save the price if there isn't already a price saved, or if the
            // new price is more recent than the existing one.
            //
            // Note: the CosmWasm implementation of Pyth contract uses the
            // price feed's `publish_time` to determine which price is newer:
            // https://github.com/pyth-network/pyth-crosschain/blob/df1ca64/target_chains/cosmwasm/contracts/pyth/src/contract.rs#L588-L589
            //
            // However, this doesn't work in practice: whereas Pyth Core prices
            // are updated every 400 ms, `publish_time` only comes in whole
            // seconds. As such, it's possible for two prices to have the same
            // `publish_time`, in which case it's impossible to tell which price
            // is newer.
            //
            // To deal with this, we addtionally compare the price feed's
            // Wormhole VAA sequence. In case `publish_time` are the same, the
            // price with the bigger sequence is accepted.
            PRICES.may_update(ctx.storage, id, |current_record| -> anyhow::Result<_> {
                match current_record {
                    Some((current_price, current_sequence)) => {
                        match current_price.timestamp.cmp(&new_price.timestamp) {
                            Ordering::Less => Ok((new_price, new_sequence)),
                            Ordering::Equal if current_sequence < new_sequence => {
                                Ok((new_price, new_sequence))
                            },
                            _ => Ok((current_price, current_sequence)),
                        }
                    },
                    None => Ok((new_price, new_sequence)),
                }
            })?;
        }
    }

    Ok(Response::new())
}
