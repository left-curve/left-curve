pub mod blocks;
pub mod events;
pub mod messages;
pub mod prelude;
pub mod transactions;

pub trait OrderByBlocks {
    fn order_by_blocks_desc(&self) -> Self;
    fn order_by_blocks_asc(&self) -> Self;
}
