pub mod blocks;
pub mod events;
pub mod messages;
pub mod prelude;
pub mod transactions;

pub trait OrderByBlocks<C> {
    fn order_by_blocks_desc(self, _phantom: std::marker::PhantomData<C>) -> Self;
    fn order_by_blocks_asc(self, _phantom: std::marker::PhantomData<C>) -> Self;
}
