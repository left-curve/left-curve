use {
    crate::Order,
    grug::{StdResult, Udec128},
    std::{cmp::Ordering, iter::Peekable},
};

pub struct MergedOrders<A, B>
where
    A: Iterator<Item = StdResult<((Udec128, u64), Order)>>,
    B: Iterator<Item = StdResult<((Udec128, u64), Order)>>,
{
    real: Peekable<A>,
    passive: Peekable<B>,
    iteration_order: grug::Order,
}

impl<A, B> MergedOrders<A, B>
where
    A: Iterator<Item = StdResult<((Udec128, u64), Order)>>,
    B: Iterator<Item = StdResult<((Udec128, u64), Order)>>,
{
    pub fn new(real: A, passive: B, iteration_order: grug::Order) -> Self {
        Self {
            real: real.peekable(),
            passive: passive.peekable(),
            iteration_order,
        }
    }
}

impl<A, B> Iterator for MergedOrders<A, B>
where
    A: Iterator<Item = StdResult<((Udec128, u64), Order)>>,
    B: Iterator<Item = StdResult<((Udec128, u64), Order)>>,
{
    type Item = StdResult<((Udec128, u64), Order)>;

    fn next(&mut self) -> Option<Self::Item> {
        match (self.real.peek(), self.passive.peek()) {
            (Some(Ok(((real_price, _), _))), Some(Ok(((passive_price, _), _)))) => {
                // Compare only the price since passive orders don't have an order ID.
                let ordering_raw = real_price.cmp(passive_price);
                let ordering = match self.iteration_order {
                    grug::Order::Ascending => ordering_raw,
                    grug::Order::Descending => ordering_raw.reverse(),
                };

                match ordering {
                    Ordering::Less => self.real.next(),
                    // In case of equal price we give the passive liquidity priority.
                    _ => self.passive.next(),
                }
            },
            (Some(Ok(_)), None) => self.real.next(),
            (None, Some(Ok(_))) => self.passive.next(),
            (Some(Err(e)), _) => Some(Err(e.clone())),
            (_, Some(Err(e))) => Some(Err(e.clone())),
            (None, None) => None,
        }
    }
}
