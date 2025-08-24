use {
    super::Prepend,
    dango_types::dex::Order,
    grug::{Order as IterationOrder, StdResult, Udec128_24},
    std::cmp::Ordering,
};

pub struct MergedOrders<A, B>
where
    A: Iterator<Item = StdResult<(Udec128_24, Order)>>,
    B: Iterator<Item = StdResult<(Udec128_24, Order)>>,
{
    a: A,
    // We don't use Rust's built-in `Peekable`, because it can't be disassembled;
    // meaning, given a `Peekable<T>`, there's no way to destroy it and get the
    // inner `T` out.
    a_peeked: Option<Option<StdResult<(Udec128_24, Order)>>>,
    b: B,
    b_peeked: Option<Option<StdResult<(Udec128_24, Order)>>>,
    /// Iterating from the lowest price to highest, or the other way around.
    iteration_order: IterationOrder,
}

impl<A, B> MergedOrders<A, B>
where
    A: Iterator<Item = StdResult<(Udec128_24, Order)>>,
    B: Iterator<Item = StdResult<(Udec128_24, Order)>>,
{
    pub fn new(a: A, b: B, iteration_order: IterationOrder) -> Self {
        Self {
            a,
            a_peeked: None,
            b,
            b_peeked: None,
            iteration_order,
        }
    }

    pub fn disassemble(self) -> (Prepend<A>, Prepend<B>) {
        (
            Prepend::new(self.a, self.a_peeked.flatten()),
            Prepend::new(self.b, self.b_peeked.flatten()),
        )
    }

    fn peek_both(
        &mut self,
    ) -> (
        Option<&StdResult<(Udec128_24, Order)>>,
        Option<&StdResult<(Udec128_24, Order)>>,
    ) {
        (
            self.a_peeked.get_or_insert_with(|| self.a.next()).as_ref(),
            self.b_peeked.get_or_insert_with(|| self.b.next()).as_ref(),
        )
    }

    fn next_a(&mut self) -> Option<StdResult<(Udec128_24, Order)>> {
        match self.a_peeked.take() {
            Some(item) => item,
            None => self.a.next(),
        }
    }

    fn next_b(&mut self) -> Option<StdResult<(Udec128_24, Order)>> {
        match self.b_peeked.take() {
            Some(item) => item,
            None => self.b.next(),
        }
    }
}

impl<A, B> Iterator for MergedOrders<A, B>
where
    A: Iterator<Item = StdResult<(Udec128_24, Order)>>,
    B: Iterator<Item = StdResult<(Udec128_24, Order)>>,
{
    type Item = StdResult<(Udec128_24, Order)>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.peek_both() {
            (Some(Ok((a_price, _))), Some(Ok((b_price, _)))) => {
                // Compare only the price since passive orders don't have an order ID.
                let ordering_raw = a_price.cmp(b_price);
                let ordering = match self.iteration_order {
                    IterationOrder::Ascending => ordering_raw,
                    IterationOrder::Descending => ordering_raw.reverse(),
                };

                match ordering {
                    // In case of equal price, pick `b`.
                    // When calling `MergedOrders::new`, the caller should ensure
                    // to put the prioritized iterator as `b`.
                    Ordering::Less => self.next_a(),
                    _ => self.next_b(),
                }
            },
            (Some(Ok(_)), None) => self.next_a(),
            (None, Some(Ok(_))) => self.next_b(),
            (None, None) => None,
            (Some(Err(err)), _) | (_, Some(Err(err))) => Some(Err(err.clone())),
        }
    }
}
