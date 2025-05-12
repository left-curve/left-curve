use {
    async_graphql::{ComplexObject, Enum, SimpleObject},
    chrono::{DateTime, TimeZone, Utc},
    dango_indexer_sql::entity,
    serde::Deserialize,
};

#[derive(Enum, Copy, Clone, Debug, Deserialize, Default, Eq, PartialEq, Hash)]
pub enum AccountType {
    #[default]
    Spot,
    Margin,
}

impl From<entity::accounts::AccountType> for AccountType {
    fn from(account_type: entity::accounts::AccountType) -> AccountType {
        match account_type {
            entity::accounts::AccountType::Spot => AccountType::Spot,
            entity::accounts::AccountType::Margin => AccountType::Margin,
        }
    }
}

#[derive(Clone, Debug, SimpleObject, Deserialize, Default, Eq, PartialEq, Hash)]
#[serde(rename_all = "camelCase")]
#[graphql(complex)]
#[serde(default)]
pub struct Account {
    pub username: String,
    pub address: String,
    // pub eth_address: Option<String>,
    pub account_type: AccountType,
    pub created_at: DateTime<Utc>,
    pub created_block_height: u64,
}

impl From<entity::accounts::Model> for Account {
    fn from(item: entity::accounts::Model) -> Self {
        Self {
            created_at: Utc.from_utc_datetime(&item.created_at),
            username: item.username,
            address: item.address,
            // eth_address: item.eth_address,
            account_type: item.account_type.into(),
            created_block_height: item.created_block_height as u64,
        }
    }
}

#[ComplexObject]
impl Account {}
