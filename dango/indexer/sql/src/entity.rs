pub mod accounts;
pub mod accounts_users;
pub mod prelude;
pub mod public_keys;
pub mod transfers;
pub mod users;

pub trait OrderByBlocks {
    fn order_by_blocks_desc(&self) -> Self;
    fn order_by_blocks_asc(&self) -> Self;
}
