use {
    grug::{declare_bounded, Bounded, Bounds, NumberConst, Udec256, Uint256},
    std::ops::Bound,
};

declare_bounded! {
    name = FeeRate,
    type = Udec256,
    min = Bound::Included(Udec256::ZERO),
    // TODO: we need an easier way of defining const Uint256
    max = Bound::Excluded(Udec256::raw(Uint256::new_from_u128(1_000_000_000_000_000_000))),
}

#[test]
fn parsing_fee_rate() {
    // Ensure the `FeeRateBounds` type is correctly defined.
    assert_eq!(FeeRateBounds::MIN, Bound::Included(Udec256::ZERO));
    assert_eq!(
        FeeRateBounds::MAX,
        Bound::Excluded(Udec256::new_percent(100_u128))
    );

    // Attempt to parse various values into `FeeRate`.
    assert!(FeeRate::new(Udec256::new_percent(0_u128)).is_ok());
    assert!(FeeRate::new(Udec256::new_percent(50_u128)).is_ok());
    assert!(FeeRate::new(Udec256::new_percent(100_u128)).is_err());
}
