use dango_types::HumanAmount;

/// Given the user's current position, decompose an order's size into closing
/// portion (that reduces existing exposure) and opening portion (that creates
/// new exposure).
///
/// Returns (closing_size, opening_size). Both have the same sign as `size`
/// (or are zero).
pub fn decompose_fill(
    size: HumanAmount,
    current_position: HumanAmount,
) -> (HumanAmount, HumanAmount) {
    // Buy order, user has short position.
    if size.is_positive() && current_position.is_negative() {
        let closing = size.min(-current_position);
        let opening = size - closing; // closing <= size, so this is guaranteed to not overflow.
        return (closing, opening);
    }

    // Sell order, user has long position.
    if size.is_negative() && current_position.is_positive() {
        let closing = size.max(-current_position);
        let opening = size - closing;
        return (closing, opening);
    }

    // Order and the current position are in the same direction: no closing, all opening.
    (HumanAmount::ZERO, size)
}

#[cfg(test)]
mod tests {
    use {super::*, dango_types::FromInner, grug::Dec128_6, test_case::test_case};

    fn human(n: i128) -> HumanAmount {
        HumanAmount::from_inner(Dec128_6::new(n))
    }

    #[test_case(   0,    0,    0,    0 ; "no order no position")]
    #[test_case(   0,   10,    0,    0 ; "no order has long")]
    #[test_case(   0,  -10,    0,    0 ; "no order has short")]
    #[test_case(  10,    0,    0,   10 ; "buy no position all opening")]
    #[test_case( -10,    0,    0,  -10 ; "sell no position all opening")]
    #[test_case(  10,    5,    0,   10 ; "buy into long all opening")]
    #[test_case( -10,   -5,    0,  -10 ; "sell into short all opening")]
    #[test_case(   5,  -10,    5,    0 ; "buy partially closes short")]
    #[test_case(  -5,   10,   -5,    0 ; "sell partially closes long")]
    #[test_case(  10,  -10,   10,    0 ; "buy exactly closes short")]
    #[test_case( -10,   10,  -10,    0 ; "sell exactly closes long")]
    #[test_case(  15,  -10,   10,    5 ; "buy closes short and opens long")]
    #[test_case( -15,   10,  -10,   -5 ; "sell closes long and opens short")]
    fn decompose_fill_works(size: i128, position: i128, exp_closing: i128, exp_opening: i128) {
        let (closing, opening) = decompose_fill(human(size), human(position));
        assert_eq!(closing, human(exp_closing));
        assert_eq!(opening, human(exp_opening));
    }
}
