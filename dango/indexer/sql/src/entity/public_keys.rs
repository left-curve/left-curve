use sea_orm::entity::prelude::*;

#[derive(EnumIter, DeriveActiveEnum, Clone, Debug, PartialEq, Eq, Hash)]
#[sea_orm(rs_type = "i32", db_type = "Integer")]
pub enum KeyType {
    #[sea_orm(num_value = 0)]
    Secp256r1,
    #[sea_orm(num_value = 1)]
    Secp256k1,
    #[sea_orm(num_value = 2)]
    Ethereum,
}

impl From<dango_types::auth::Key> for KeyType {
    fn from(key: dango_types::auth::Key) -> Self {
        match key {
            dango_types::auth::Key::Secp256r1(_) => KeyType::Secp256r1,
            dango_types::auth::Key::Secp256k1(_) => KeyType::Secp256k1,
            dango_types::auth::Key::Ethereum(_) => KeyType::Ethereum,
        }
    }
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Hash)]
#[sea_orm(table_name = "users_public_keys")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub username: String,
    pub key_hash: String,
    pub public_key: String,
    pub key_type: KeyType,
    pub created_at: DateTime,
    pub created_block_height: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::users::Entity",
        from = "Column::Username",
        to = "super::users::Column::Username"
    )]
    User,
}

impl Related<super::users::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
