use async_graphql::SimpleObject;

#[derive(SimpleObject)]
pub struct Store {
    /// The base64 encoded value
    pub value: String,
    /// The base64 encoded proof
    pub proof: Option<String>,
}
