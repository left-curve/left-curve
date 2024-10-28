use {
    crate::{CONFIG, GUARDIANS, PRICES, PRICE_FEEDS},
    anyhow::ensure,
    dango_types::oracle::{ExecuteMsg, InstantiateMsg, Price},
    grug::{Denom, Integer, MutableCtx, Number, Order, Response, StdResult, SudoCtx, Udec128},
    std::collections::BTreeMap,
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> StdResult<Response> {
    CONFIG.save(ctx.storage, &msg.config)?;

    for guardian in msg.guardians {
        GUARDIANS.insert(ctx.storage, guardian)?;
    }

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn execute(ctx: MutableCtx, msg: ExecuteMsg) -> anyhow::Result<Response> {
    match msg {
        ExecuteMsg::FeedPrices(feeds) => feed_prices(ctx, feeds),
    }
}

fn feed_prices(ctx: MutableCtx, feeds: BTreeMap<Denom, Udec128>) -> anyhow::Result<Response> {
    // Only guardians can feed prices.
    ensure!(
        GUARDIANS.has(ctx.storage, ctx.sender),
        "you don't have the right, O you don't have the right"
    );

    // Save the prices in storage.
    // If the guardian has fed prices before in the same epoch, the older feeds
    // are simply overwritten.
    for (denom, feed) in feeds {
        PRICE_FEEDS.save(ctx.storage, (&denom, ctx.sender), &feed)?;
    }

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn cron_execute(ctx: SudoCtx) -> StdResult<Response> {
    let cfg = CONFIG.load(ctx.storage)?;

    let mut denom_feeds = BTreeMap::<Denom, Vec<Udec128>>::new();

    // Load all feeds submitted by guardians since the last cron execute.
    for res in PRICE_FEEDS.prefix_range(ctx.storage, None, None, Order::Ascending) {
        let ((denom, _guardian), feed) = res?;

        // We expect there to be ~20 guardians, so pre-allocate a vector
        // with capacity of 20.
        denom_feeds
            .entry(denom)
            .or_insert_with(|| Vec::with_capacity(20))
            .push(feed);
    }

    // Compute price for each denom based on the feeds.
    for (denom, mut feeds) in denom_feeds {
        let num_feeds = feeds.len();

        // If there's not enough feeds, then don't compute the median.
        if num_feeds < cfg.quorum as usize {
            continue;
        }

        // The price feeds must be sorted in order to find the median.
        feeds.sort();

        // Find the median price.
        // Here as an optimization, we use bitwise operators (`&` and `>>`)
        // instead of division or modulo.
        let median = if num_feeds & 1 == 0 {
            // If there's an even number of feeds, take the average of the two
            // middle ones.
            let feed1 = feeds[(num_feeds >> 1) - 1];
            let feed2 = feeds[num_feeds >> 1];

            feed1
                .numerator()
                .checked_add(*feed2.numerator())?
                .checked_shr(1)
                .map(Udec128::raw)?
        } else {
            feeds[num_feeds >> 1]
        };

        PRICES.save(ctx.storage, &denom, &Price {
            price: median,
            timestamp: ctx.block.timestamp,
        })?;
    }

    // Delete all feeds. Start with a clean slate for the next epoch.
    PRICE_FEEDS.clear(ctx.storage, None, None);

    Ok(Response::new())
}
