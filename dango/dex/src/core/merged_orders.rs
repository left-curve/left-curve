use {
    dango_types::dex::Order,
    grug::{Order as IterationOrder, StdResult, Udec128_24},
    std::{cmp::Ordering, iter::Peekable},
};

pub struct MergedOrders<A, B, C>
where
    A: Iterator<Item = StdResult<(Udec128_24, Order)>>,
    B: Iterator<Item = StdResult<(Udec128_24, Order)>>,
    C: Iterator<Item = StdResult<(Udec128_24, Order)>>,
{
    a: Peekable<A>,
    b: Peekable<B>,
    c: Peekable<C>,
    /// Iterating from the lowest price to highest, or the other way around.
    iteration_order: IterationOrder,
}

impl<A, B, C> MergedOrders<A, B, C>
where
    A: Iterator<Item = StdResult<(Udec128_24, Order)>>,
    B: Iterator<Item = StdResult<(Udec128_24, Order)>>,
    C: Iterator<Item = StdResult<(Udec128_24, Order)>>,
{
    pub fn new(a: A, b: B, c: C, iteration_order: IterationOrder) -> Self {
        Self {
            a: a.peekable(),
            b: b.peekable(),
            c: c.peekable(),
            iteration_order,
        }
    }

    pub fn disassemble(self) -> (Peekable<A>, Peekable<B>, Peekable<C>) {
        (self.a, self.b, self.c)
    }
}

impl<A, B, C> Iterator for MergedOrders<A, B, C>
where
    A: Iterator<Item = StdResult<(Udec128_24, Order)>>,
    B: Iterator<Item = StdResult<(Udec128_24, Order)>>,
    C: Iterator<Item = StdResult<(Udec128_24, Order)>>,
{
    type Item = StdResult<(Udec128_24, Order)>;

    fn next(&mut self) -> Option<Self::Item> {
        // Note: if prices are the same, priority is c > b > a.
        match (self.a.peek(), self.b.peek(), self.c.peek()) {
            (Some(Ok((a_price, _))), Some(Ok((b_price, _))), Some(Ok((c_price, _)))) => {
                match compare(a_price, b_price, self.iteration_order) {
                    // a is prioritized over b. Now compare a and c.
                    Ordering::Less => match compare(a_price, c_price, self.iteration_order) {
                        Ordering::Less => self.a.next(),
                        _ => self.c.next(),
                    },
                    // b is prioritized over a. Now compare b and c.
                    _ => match compare(b_price, c_price, self.iteration_order) {
                        Ordering::Less => self.b.next(),
                        _ => self.c.next(),
                    },
                }
            },
            (Some(Ok((a_price, _))), Some(Ok((b_price, _))), None) => {
                match compare(a_price, b_price, self.iteration_order) {
                    // In case of equal price, priority b > a.
                    Ordering::Less => self.a.next(),
                    _ => self.b.next(),
                }
            },
            (Some(Ok((a_price, _))), None, Some(Ok((c_price, _)))) => {
                match compare(a_price, c_price, self.iteration_order) {
                    // In case of equal price, priority c > a.
                    Ordering::Less => self.a.next(),
                    _ => self.c.next(),
                }
            },
            (None, Some(Ok((b_price, _))), Some(Ok((c_price, _)))) => {
                match compare(b_price, c_price, self.iteration_order) {
                    // In case of equal price, priority c > b.
                    Ordering::Less => self.b.next(),
                    _ => self.c.next(),
                }
            },
            (Some(Ok(_)), None, None) => self.a.next(),
            (None, Some(Ok(_)), None) => self.b.next(),
            (None, None, Some(Ok(_))) => self.c.next(),
            (None, None, None) => None,
            (Some(Err(err)), ..) | (_, Some(Err(err)), _) | (_, _, Some(Err(err))) => {
                Some(Err(err.clone()))
            },
        }
    }
}

fn compare(p1: &Udec128_24, p2: &Udec128_24, iteration_order: IterationOrder) -> Ordering {
    let ordering_raw = p1.cmp(p2);
    match iteration_order {
        IterationOrder::Ascending => ordering_raw,
        IterationOrder::Descending => ordering_raw.reverse(),
    }
}
