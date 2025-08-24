use {
    dango_types::dex::Direction,
    grug::{MathResult, NonZero, Number, Udec128_24},
};

/// Find the price bucket that an order belongs to.
///
/// Price buckets are determined by bucket sizes. E.g., a bucket size of 100
/// defines the buckets 100, 200, 300...The question is what to do an order
/// whose price is not a multiple of the bucket size. Let's consider an order
/// with price $150.
///
/// For a bid (a BUY order) at $150, it is only relevant for sellers whose
/// prices are _lower_ than $150, so it's rounded down, to the $100 bucket.
///
/// For a ask (a SELL order) at $150, it is only relevant for buyers whose
/// prices are _higher_ than $150, so it's rounded up, to the $200 bucket.
pub fn bucket(
    price: Udec128_24,
    direction: Direction,
    bucket_size: NonZero<Udec128_24>,
) -> MathResult<Udec128_24> {
    let floored = price.checked_div(*bucket_size)?.checked_mul(*bucket_size)?;
    match direction {
        Direction::Ask if floored < price => floored.checked_add(*bucket_size),
        _ => Ok(floored),
    }
}
