use {
    crate::{state::GUARDIAN_SETS, PRICE_SOURCES},
    anyhow::ensure,
    dango_types::oracle::{ExecuteMsg, InstantiateMsg, PriceSource, PythId, PythVaa, PRICES},
    grug::{Binary, Denom, Inner, MutableCtx, Response},
    std::collections::BTreeMap,
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> anyhow::Result<Response> {
    for (i, guardian_set) in msg.guardian_sets {
        GUARDIAN_SETS.save(ctx.storage, i, &guardian_set)?;
    }

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn execute(ctx: MutableCtx, msg: ExecuteMsg) -> anyhow::Result<Response> {
    match msg {
        ExecuteMsg::RegisterPriceSources(price_sources) => {
            register_price_sources(ctx, price_sources)
        },
        ExecuteMsg::FeedPrices(vaas) => feed_prices(ctx, vaas),
    }
}

fn register_price_sources(
    ctx: MutableCtx,
    price_sources: BTreeMap<Denom, PriceSource>,
) -> anyhow::Result<Response> {
    let cfg = ctx.querier.query_config()?;

    // Only chain owner can register a denom.
    ensure!(
        ctx.sender == cfg.owner,
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

        // Verify the VAA, and store the prices.
        for feed in vaa.verify(ctx.storage, ctx.api, ctx.block, GUARDIAN_SETS)? {
            let hash = PythId::from_inner(feed.id.to_bytes());

            // Save the price if there isn't already a price saved, or if there
            // is but it's older.
            PRICES.may_update(ctx.storage, hash, |maybe_price| -> anyhow::Result<_> {
                if let Some(price) = maybe_price {
                    if price.timestamp < feed.get_price_unchecked().publish_time as u64 {
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
