use {
    dango_types::dex::{CreateLimitOrderRequest, Direction},
    grug::{Coins, Denom, MultiplyFraction, NonZero, Udec128_24, Uint128},
};

pub fn create_limit_order_request(
    base_denom: Denom,
    quote_denom: Denom,
    direction: Direction,
    amount_base: Uint128,
    price: Udec128_24,
) -> (Coins, CreateLimitOrderRequest) {
    match direction {
        Direction::Bid => {
            let amount_quote = amount_base.checked_mul_dec_ceil(price).unwrap();
            (
                Coins::one(quote_denom.clone(), amount_quote).unwrap(),
                CreateLimitOrderRequest::Bid {
                    base_denom,
                    quote_denom,
                    amount_quote: NonZero::new(amount_quote).unwrap(),
                    price: NonZero::new(price).unwrap(),
                },
            )
        },
        Direction::Ask => (
            Coins::one(base_denom.clone(), amount_base).unwrap(),
            CreateLimitOrderRequest::Ask {
                base_denom,
                quote_denom,
                amount_base: NonZero::new(amount_base).unwrap(),
                price: NonZero::new(price).unwrap(),
            },
        ),
    }
}
