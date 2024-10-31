use {
    crate::{query_price, state::GUARDIAN_SETS, PRICE_SOURCES},
    anyhow::ensure,
    dango_types::oracle::{
        ExecuteMsg, InstantiateMsg, PriceSourceCollector, PythId, PythVaa, QueryMsg, PRICE_FEEDS,
    },
    grug::{Attribute, Denom, ImmutableCtx, Json, JsonSerExt, MutableCtx, Response},
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> anyhow::Result<Response> {
    for (i, guardian_set) in msg.guardian_set {
        GUARDIAN_SETS.save(ctx.storage, i, &guardian_set)?;
    }

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> anyhow::Result<Json> {
    match msg {
        QueryMsg::QueryPrice { denom } => {
            let res = query_price(ctx, denom)?;
            Ok(res.to_json_value()?)
        },
    }
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn execute(ctx: MutableCtx, msg: ExecuteMsg) -> anyhow::Result<Response> {
    match msg {
        ExecuteMsg::UpdatePriceFeeds { data } => update_price_feeds(ctx, data),
        ExecuteMsg::RegisterDenom {
            denom,
            price_source,
        } => register_price_source(ctx, denom, price_source),
    }
}

fn update_price_feeds(ctx: MutableCtx, vaas: Vec<PythVaa>) -> anyhow::Result<Response> {
    let feeds =
        vaas.into_iter()
            .try_fold(vec![], |mut feeds, pyth_vaa| -> anyhow::Result<Vec<_>> {
                // In case one vaa is not valid, should we skip it or abort the whole transaction?
                feeds.extend(pyth_vaa.verify(ctx.storage, ctx.api, ctx.block, GUARDIAN_SETS)?);
                Ok(feeds)
            })?;

    let attrs =
        feeds
            .into_iter()
            .try_fold(vec![], |mut attrs, new_feed| -> anyhow::Result<Vec<_>> {
                let hash = PythId::from_inner(new_feed.id.to_bytes());

                let mut updated: bool = true;

                PRICE_FEEDS.may_update(ctx.storage, hash, |a| -> anyhow::Result<_> {
                    if let Some(current_feed) = a {
                        if current_feed.timestamp
                            < new_feed.get_price_unchecked().publish_time as u64
                        {
                            new_feed.try_into()
                        } else {
                            updated = false;
                            Ok(current_feed)
                        }
                    } else {
                        new_feed.try_into()
                    }
                })?;

                if updated {
                    attrs.push(Attribute::new(hash, new_feed.to_json_string_pretty()?));
                }

                Ok(attrs)
            })?;

    Ok(Response::new().add_attributes(attrs))
}

fn register_price_source(
    ctx: MutableCtx,
    denom: Denom,
    price_source: PriceSourceCollector,
) -> anyhow::Result<Response> {
    let cfg = ctx.querier.query_config()?;

    // Only chain owner can register a denom.
    ensure!(
        ctx.sender == cfg.owner,
        "you don't have the right, O you don't have the right"
    );

    PRICE_SOURCES.save(ctx.storage, &denom, &price_source)?;

    Ok(Response::new())
}
