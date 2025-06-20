pub mod blocks;
pub mod events;
pub mod messages;
pub mod prelude;
pub mod transactions;

pub trait OrderByBlocks<C> {
    // fn order_by_blocks_desc(self) -> Self;
    // fn order_by_blocks_asc(self) -> Self;

    fn order_by_blocks(self, order: sea_orm::Order) -> Self;

    fn phantom(_phantom: std::marker::PhantomData<C>) {}
}
