use async_graphql::Object;

#[derive(Default, Debug)]
pub struct BlockQuery {}

#[Object]
impl BlockQuery {
    /// Get a block by its height
    async fn block(&self, _height: u64) -> Option<u64> {
        None
    }
}
