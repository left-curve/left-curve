use {
    dango_types::dex::Order,
    grug::{Order as IterationOrder, Udec128_24},
    std::{cmp::Ordering, iter::Peekable},
};

pub struct MergedOrders<A, B>
where
    A: Iterator<Item = (Udec128_24, Order)>,
    B: Iterator<Item = (Udec128_24, Order)>,
{
    a: Peekable<A>,
    b: Peekable<B>,
    /// Iterating from the lowest price to highest, or the other way around.
    iteration_order: IterationOrder,
}

impl<A, B> MergedOrders<A, B>
where
    A: Iterator<Item = (Udec128_24, Order)>,
    B: Iterator<Item = (Udec128_24, Order)>,
{
    pub fn new(a: A, b: B, iteration_order: IterationOrder) -> Self {
        Self {
            a: a.peekable(),
            b: b.peekable(),
            iteration_order,
        }
    }

    pub fn disassemble(self) -> (Peekable<A>, Peekable<B>) {
        (self.a, self.b)
    }
}

impl<A, B> Iterator for MergedOrders<A, B>
where
    A: Iterator<Item = (Udec128_24, Order)>,
    B: Iterator<Item = (Udec128_24, Order)>,
{
    type Item = (Udec128_24, Order);

    fn next(&mut self) -> Option<Self::Item> {
        match (self.a.peek(), self.b.peek()) {
            (Some((a_price, _)), Some((b_price, _))) => {
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
                    Ordering::Less => self.a.next(),
                    _ => self.b.next(),
                }
            },
            (Some(_), None) => self.a.next(),
            (None, Some(_)) => self.b.next(),
            (None, None) => None,
        }
    }
}
