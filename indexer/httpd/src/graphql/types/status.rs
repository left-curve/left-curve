use {super::block::BlockInfo, async_graphql::SimpleObject};

#[derive(SimpleObject)]
pub struct Status {
    pub block: BlockInfo,
    pub chain_id: String,
}
