use {
    ::metrics::{describe_counter, describe_gauge, describe_histogram},
    dango_types::{dex::Price, oracle::PrecisionedPrice},
    grug::{CoinPair, Denom, Inner, Number, Udec128_6, Uint128},
    std::{collections::HashMap, sync::Once},
};

pub const LABEL_TRADES: &str = "dango.contract.dex.trades_count";

pub const LABEL_ORDERS_FILLED: &str = "dango.contract.dex.orders_filled_count";

pub const LABEL_CONTRACT_AMOUNT: &str = "dango.contract.dex.contract_amount";

pub const LABEL_CONTRACT_VALUE: &str = "dango.contract.dex.contract_value";

pub const LABEL_RESERVE_AMOUNT: &str = "dango.contract.dex.reserve_amount";

pub const LABEL_RESERVE_VALUE: &str = "dango.contract.dex.reserve_value";

pub const LABEL_TRADES_PER_BLOCK: &str = "dango.contract.dex.trades_per_block";

pub const LABEL_VOLUME_PER_TRADE: &str = "dango.contract.dex.volume_per_trade";

pub const LABEL_VOLUME_AMOUNT_PER_BLOCK: &str = "dango.contract.dex.volume_amount_per_block";

pub const LABEL_VOLUME_VALUE_PER_BLOCK: &str = "dango.contract.dex.volume_value_per_block";

pub const LABEL_BEST_PRICE: &str = "dango.contract.dex.best_price";

pub const LABEL_SPREAD_ABSOLUTE: &str = "dango.contract.dex.spread_absolute";

pub const LABEL_SPREAD_PERCENTAGE: &str = "dango.contract.dex.spread_percentage";

pub const LABEL_DURATION_AUCTION: &str = "dango.contract.dex.auction.duration";

pub const LABEL_DURATION_REFLECT_CURVE: &str = "dango.contract.dex.reflect_curve.duration";

pub const LABEL_DURATION_ORDER_MATCHING: &str = "dango.contract.dex.order_matching.duration";

pub const LABEL_DURATION_ORDER_FILLING: &str = "dango.contract.dex.order_filling.duration";

pub const LABEL_DURATION_CANCEL_IOC: &str = "dango.contract.dex.cancel_ioc.duration";

pub const LABEL_DURATION_UPDATE_REST_STATE: &str = "dango.contract.dex.update_rest_state.duration";

pub const LABEL_DURATION_STORE_VOLUME: &str = "dango.contract.dex.store_volume.duration";

pub const LABEL_DURATION_ITER_NEXT: &str = "dango.contract.dex.iterator_next.duration";

pub fn init_metrics() {
    static ONCE: Once = Once::new();

    ONCE.call_once(|| {
        describe_counter!(LABEL_TRADES, "Number of trades executed");

        describe_counter!(LABEL_ORDERS_FILLED, "Number of unique orders filled");

        describe_gauge!(LABEL_CONTRACT_AMOUNT, "Amount of coin in the dex contract");

        describe_gauge!(LABEL_CONTRACT_VALUE, "Value of dex contract");

        describe_gauge!(LABEL_RESERVE_AMOUNT, "Amount of coin in the reserve");

        describe_gauge!(LABEL_RESERVE_VALUE, "Value of reserve");

        describe_histogram!(LABEL_TRADES_PER_BLOCK, "Number of trades in a block");

        describe_histogram!(LABEL_VOLUME_PER_TRADE, "Volume per trade");

        describe_histogram!(
            LABEL_VOLUME_AMOUNT_PER_BLOCK,
            "Volume per block in token amount"
        );

        describe_histogram!(
            LABEL_VOLUME_VALUE_PER_BLOCK,
            "Volume per block in token value"
        );

        describe_gauge!(LABEL_BEST_PRICE, "Best price available in order book");

        describe_gauge!(LABEL_SPREAD_ABSOLUTE, "Absolute spread between bid and ask");

        describe_gauge!(
            LABEL_SPREAD_PERCENTAGE,
            "Percentage spread between bid and ask"
        );

        describe_histogram!(
            LABEL_DURATION_AUCTION,
            "Time spent on the entire auction across all pairs"
        );

        describe_histogram!(
            LABEL_DURATION_REFLECT_CURVE,
            "Time spent on reflecting passive liquidity pool curve"
        );

        describe_histogram!(
            LABEL_DURATION_ORDER_MATCHING,
            "Time spent on matching orders"
        );

        describe_histogram!(LABEL_DURATION_ORDER_FILLING, "Time spent on filling orders");

        describe_histogram!(
            LABEL_DURATION_CANCEL_IOC,
            "Time spent on canceling IOC orders"
        );

        describe_histogram!(
            LABEL_DURATION_UPDATE_REST_STATE,
            "Time spent on updating the resting order book state"
        );

        describe_histogram!(LABEL_DURATION_STORE_VOLUME, "Time spent on storing volume");

        describe_histogram!(
            LABEL_DURATION_ITER_NEXT,
            "Time spent on advancing an iterator"
        );
    });
}

pub fn reserve(
    base_denom: &Denom,
    quote_denom: &Denom,
    base_price: &PrecisionedPrice,
    quote_price: &PrecisionedPrice,
    reserve: CoinPair,
) -> anyhow::Result<()> {
    for coin in reserve.into_iter() {
        let price = if &coin.denom == base_denom {
            base_price
        } else {
            quote_price
        };

        // Divide the amount by 10^precision to get the human-readable amount.
        let scale_f64 = 10_f64.powi(price.precision() as i32);
        let amount_f64 = (coin.amount.into_inner() as f64) / scale_f64;

        // Amount of tokens in reserve.
        ::metrics::gauge!(crate::metrics::LABEL_RESERVE_AMOUNT,
            "base_denom" => base_denom.to_string(),
            "quote_denom" => quote_denom.to_string(),
            "token" => coin.denom.to_string()
        )
        .set(amount_f64);

        // Value of tokens in reserve (USD).
        let value: Udec128_6 = price.value_of_unit_amount(coin.amount)?;
        let value_f64: f64 = value.to_string().parse()?;

        ::metrics::gauge!(crate::metrics::LABEL_RESERVE_VALUE,
            "base_denom" => base_denom.to_string(),
            "quote_denom" => quote_denom.to_string(),
            "token" => coin.denom.to_string()
        )
        .set(value_f64);
    }

    Ok(())
}

pub fn volume(
    base_denom: &Denom,
    quote_denom: &Denom,
    base_price: &PrecisionedPrice,
    quote_price: &PrecisionedPrice,
    volume_data: HashMap<&Denom, Uint128>,
) -> anyhow::Result<()> {
    for (token, amount) in volume_data {
        let price = if token == base_denom {
            &base_price
        } else {
            &quote_price
        };

        let scale_f64 = 10_f64.powi(price.precision() as i32);

        let amount_f64 = (amount.into_inner() as f64) / scale_f64;

        let value: Udec128_6 = price.value_of_unit_amount(amount)?;
        let value_f64: f64 = value.to_string().parse()?;

        ::metrics::histogram!(
            crate::metrics::LABEL_VOLUME_AMOUNT_PER_BLOCK,
            "base_denom" => base_denom.to_string(),
            "quote_denom" => quote_denom.to_string(),
            "token" => token.to_string(),
        )
        .record(amount_f64);

        ::metrics::histogram!(
            crate::metrics::LABEL_VOLUME_VALUE_PER_BLOCK,
            "base_denom" => base_denom.to_string(),
            "quote_denom" => quote_denom.to_string(),
            "token" => token.to_string(),
        )
        .record(value_f64);
    }

    Ok(())
}

pub fn best_price(
    base_denom: &Denom,
    quote_denom: &Denom,
    base_price: &PrecisionedPrice,
    quote_price: &PrecisionedPrice,
    best_bid_price: Option<Price>,
    best_ask_price: Option<Price>,
    mid_price: Option<Price>,
) -> anyhow::Result<()> {
    let scale_f64 = 10_f64.powi(base_price.precision() as i32 - quote_price.precision() as i32);

    if let Some(bid) = best_bid_price {
        let bid_price_f64: f64 = bid.to_string().parse()?;

        ::metrics::gauge!(crate::metrics::LABEL_BEST_PRICE,
            "base_denom" => base_denom.to_string(),
            "quote_denom" => quote_denom.to_string(),
            "type" => "bid",
        )
        .set(bid_price_f64 * scale_f64);
    }

    if let Some(ask) = best_ask_price {
        let ask_price_f64: f64 = ask.to_string().parse()?;

        ::metrics::gauge!(crate::metrics::LABEL_BEST_PRICE,
            "base_denom" => base_denom.to_string(),
            "quote_denom" => quote_denom.to_string(),
            "type" => "ask",
        )
        .set(ask_price_f64 * scale_f64);
    }

    if let Some(mid) = mid_price {
        let mid_price_f64: f64 = mid.to_string().parse()?;

        ::metrics::gauge!(crate::metrics::LABEL_BEST_PRICE,
            "base_denom" => base_denom.to_string(),
            "quote_denom" => quote_denom.to_string(),
            "type" => "mid",
        )
        .set(mid_price_f64 * scale_f64);
    }

    Ok(())
}

pub fn spread(
    base_denom: &Denom,
    quote_denom: &Denom,
    base_price: &PrecisionedPrice,
    quote_price: &PrecisionedPrice,
    best_bid_price: Option<Price>,
    best_ask_price: Option<Price>,
    mid_price: Option<Price>,
) -> anyhow::Result<()> {
    let scale_f64 = 10_f64.powi(base_price.precision() as i32 - quote_price.precision() as i32);

    if let (Some(bid), Some(ask), Some(mid)) = (best_bid_price, best_ask_price, mid_price) {
        let spread_absolute = ask - bid;

        let mut spread_absolute_f64: f64 = spread_absolute.to_string().parse()?;

        // The spread absolute needs to be adjusted according to difference in the tokens's precision.
        spread_absolute_f64 *= scale_f64;

        let spread_percentage_f64: f64 = spread_absolute.checked_div(mid)?.to_string().parse()?;

        ::metrics::gauge!(crate::metrics::LABEL_SPREAD_ABSOLUTE,
            "base_denom" => base_denom.to_string(),
            "quote_denom" => quote_denom.to_string(),
        )
        .set(spread_absolute_f64);

        ::metrics::gauge!(crate::metrics::LABEL_SPREAD_PERCENTAGE,
            "base_denom" => base_denom.to_string(),
            "quote_denom" => quote_denom.to_string(),
        )
        .set(spread_percentage_f64);
    }

    Ok(())
}
