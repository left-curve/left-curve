use {
    async_graphql::{ComplexObject, SimpleObject},
    chrono::{DateTime, Utc},
    serde::Deserialize,
};

#[derive(Clone, Debug, SimpleObject, Deserialize, Default, Eq, PartialEq, Hash)]
#[serde(rename_all = "camelCase")]
#[graphql(complex)]
#[serde(default)]
pub struct Transfer {
    pub block_height: i64,
    pub created_at: DateTime<Utc>,
    pub from_address: String,
    pub to_address: String,
    pub amount: String,
    pub denom: String,
}

// TODO: Once subindexer is merged
// impl From<entity::transfers::Model> for Block {
//    fn from(item: entity::transfers::Model) -> Self {
//        Self {
//            block_height: item.block_height,
//            created_at: Utc.from_utc_datetime(&item.created_at),
//        }
//    }
//}

#[ComplexObject]
impl Transfer {}
