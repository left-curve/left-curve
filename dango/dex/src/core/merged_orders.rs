use {
    crate::Order,
    grug::{Addr, Order as IterationOrder, StdResult, Udec128, Uint128},
    std::{cmp::Ordering, iter::Peekable},
};

const PASSIVE_ORDER_ID: u64 = 0;

// Use block height 0, so that the passive pool is always charged the maker rate.
const PASSIVE_ORDER_CREATION_BLOCK_HEIGHT: u64 = 0;

pub struct MergedOrders<A, B>
where
    A: Iterator<Item = StdResult<((Udec128, u64), Order)>>,
    B: Iterator<Item = (Udec128, Uint128)>,
{
    real: Peekable<A>,
    passive: Peekable<B>,
    iteration_order: IterationOrder,
    dex: Addr,
}

impl<A, B> MergedOrders<A, B>
where
    A: Iterator<Item = StdResult<((Udec128, u64), Order)>>,
    B: Iterator<Item = (Udec128, Uint128)>,
{
    pub fn new(real: A, passive: B, iteration_order: IterationOrder, dex: Addr) -> Self {
        Self {
            real: real.peekable(),
            passive: passive.peekable(),
            iteration_order,
            dex,
        }
    }

    fn next_passive(&mut self) -> Option<StdResult<((Udec128, u64), Order)>> {
        self.passive.next().map(|(price, amount)| {
            let order = Order {
                user: self.dex,
                amount,
                remaining: amount,
                created_at_block_height: PASSIVE_ORDER_CREATION_BLOCK_HEIGHT,
            };

            Ok(((price, PASSIVE_ORDER_ID), order))
        })
    }
}

impl<A, B> Iterator for MergedOrders<A, B>
where
    A: Iterator<Item = StdResult<((Udec128, u64), Order)>>,
    B: Iterator<Item = (Udec128, Uint128)>,
{
    type Item = StdResult<((Udec128, u64), Order)>;

    fn next(&mut self) -> Option<Self::Item> {
        match (self.real.peek(), self.passive.peek()) {
            (Some(Ok(((real_price, _), _))), Some((passive_price, _))) => {
                // Compare only the price since passive orders don't have an order ID.
                let ordering_raw = real_price.cmp(passive_price);
                let ordering = match self.iteration_order {
                    IterationOrder::Ascending => ordering_raw,
                    IterationOrder::Descending => ordering_raw.reverse(),
                };

                match ordering {
                    Ordering::Less => self.real.next(),
                    // In case of equal price we give the passive liquidity priority.
                    _ => self.next_passive(),
                }
            },
            (Some(Ok(_)), None) => self.real.next(),
            (None, Some(_)) => self.next_passive(),
            (None, None) => None,
            (Some(Err(e)), _) => Some(Err(e.clone())),
        }
    }
}
