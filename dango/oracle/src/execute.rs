use {
    crate::{state::GUARDIAN_SETS, PRICE_SOURCES},
    anyhow::ensure,
    dango_types::oracle::{ExecuteMsg, InstantiateMsg, PriceSource, PythId, PythVaa, PRICES},
    grug::{Denom, MutableCtx, Response},
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
        ExecuteMsg::FeedPrices(vaas) => update_price_feeds(ctx, vaas),
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

fn update_price_feeds(ctx: MutableCtx, vaas: Vec<PythVaa>) -> anyhow::Result<Response> {
    let feeds =
        vaas.into_iter()
            .try_fold(vec![], |mut feeds, pyth_vaa| -> anyhow::Result<Vec<_>> {
                // In case one vaa is not valid, should we skip it or abort the whole transaction?
                feeds.extend(pyth_vaa.verify(ctx.storage, ctx.api, ctx.block, GUARDIAN_SETS)?);
                Ok(feeds)
            })?;

    for new_feed in feeds {
        let hash = PythId::from_inner(new_feed.id.to_bytes());

        PRICES.may_update(ctx.storage, hash, |a| -> anyhow::Result<_> {
            if let Some(current_feed) = a {
                if current_feed.timestamp < new_feed.get_price_unchecked().publish_time as u64 {
                    new_feed.try_into()
                } else {
                    Ok(current_feed)
                }
            } else {
                new_feed.try_into()
            }
        })?;
    }

    Ok(Response::new())
}
