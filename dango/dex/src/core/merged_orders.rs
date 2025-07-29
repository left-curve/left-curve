use {
    crate::{LimitOrder, Order, PassiveOrder},
    grug::{Order as IterationOrder, StdResult, Udec128_24},
    std::{cmp::Ordering, iter::Peekable},
};

pub struct MergedOrders<A, B>
where
    A: Iterator<Item = StdResult<(Udec128_24, LimitOrder)>>,
    B: Iterator<Item = (Udec128_24, PassiveOrder)>,
{
    /// Iterator that returns real orders in the form of `(price, limit_order)`.
    real: Peekable<A>,
    /// Iterator that returns passive orders in the form of `(price, amount)`.
    passive: Peekable<B>,
    /// Iterating from the lowest price to highest, or the other way around.
    iteration_order: IterationOrder,
}

impl<A, B> MergedOrders<A, B>
where
    A: Iterator<Item = StdResult<(Udec128_24, LimitOrder)>>,
    B: Iterator<Item = (Udec128_24, PassiveOrder)>,
{
    pub fn new(real: A, passive: B, iteration_order: IterationOrder) -> Self {
        Self {
            real: real.peekable(),
            passive: passive.peekable(),
            iteration_order,
        }
    }

    fn next_real(&mut self) -> Option<StdResult<(Udec128_24, Order)>> {
        self.real.next().map(|res| {
            let (price, limit_order) = res?;
            let order = Order::Limit(limit_order);
            Ok((price, order))
        })
    }

    fn next_passive(&mut self) -> Option<StdResult<(Udec128_24, Order)>> {
        self.passive.next().map(|(price, passive_order)| {
            let order = Order::Passive(passive_order);
            Ok((price, order))
        })
    }
}

impl<A, B> Iterator for MergedOrders<A, B>
where
    A: Iterator<Item = StdResult<(Udec128_24, LimitOrder)>>,
    B: Iterator<Item = (Udec128_24, PassiveOrder)>,
{
    type Item = StdResult<(Udec128_24, Order)>;

    fn next(&mut self) -> Option<Self::Item> {
        match (self.real.peek(), self.passive.peek()) {
            (Some(Ok((real_price, _))), Some((passive_price, _))) => {
                // Compare only the price since passive orders don't have an order ID.
                let ordering_raw = real_price.cmp(passive_price);
                let ordering = match self.iteration_order {
                    IterationOrder::Ascending => ordering_raw,
                    IterationOrder::Descending => ordering_raw.reverse(),
                };

                match ordering {
                    Ordering::Less => self.next_real(),
                    // In case of equal price we give the passive liquidity priority.
                    _ => self.next_passive(),
                }
            },
            (Some(Ok(_)), None) => self.next_real(),
            (None, Some(_)) => self.next_passive(),
            (None, None) => None,
            (Some(Err(e)), _) => Some(Err(e.clone())),
        }
    }
}
