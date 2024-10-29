use {
    crate::{
        query_price_feed,
        state::{GUARDIAN_SETS, PRICE_FEEDS},
    },
    dango_types::oracle::{ExecuteMsg, InstantiateMsg, PythId, PythVaa, QueryMsg},
    grug::{Attribute, ImmutableCtx, Json, JsonSerExt, MutableCtx, Response, StdResult},
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> anyhow::Result<Response> {
    for (i, guardian_set) in msg.guardian_set {
        GUARDIAN_SETS.save(ctx.storage, i, &guardian_set)?;
    }

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> StdResult<Json> {
    match msg {
        QueryMsg::PriceFeed { id } => {
            let res = query_price_feed(ctx, id)?;
            res.to_json_value()
        },
    }
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn execute(ctx: MutableCtx, msg: ExecuteMsg) -> anyhow::Result<Response> {
    match msg {
        ExecuteMsg::UpdatePriceFeeds { data } => update_price_feeds(ctx, data),
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
                        if current_feed.get_price_unchecked().publish_time
                            < new_feed.get_price_unchecked().publish_time
                        {
                            Ok(new_feed)
                        } else {
                            updated = false;
                            Ok(current_feed)
                        }
                    } else {
                        Ok(new_feed)
                    }
                })?;

                if updated {
                    attrs.push(Attribute::new(hash, new_feed.to_json_string_pretty()?));
                }

                Ok(attrs)
            })?;

    Ok(Response::new().add_attributes(attrs))
}
