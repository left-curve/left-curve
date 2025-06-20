pub mod blocks;
pub mod events;
pub mod messages;
pub mod prelude;
pub mod transactions;

pub trait OrderByBlocks<C> {
    fn order_by_blocks(self, order: sea_orm::Order) -> Self;

    // Type parameter C is used to avoid orphan rule violations when implementing
    // this trait for external types (like SeaORM Select<E>) in other crates.
    // The phantom function ensures the type parameter is considered "used" by the trait.
    // NOTE: I tried many other ways to avoid this, but this is the only one that works.
    fn phantom(_phantom: std::marker::PhantomData<C>) {}
}
