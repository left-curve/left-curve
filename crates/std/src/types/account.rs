use {
    crate::{Addr, Hash},
    serde::{Deserialize, Serialize},
};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Account {
    pub code_hash: Hash,
    pub admin:     Option<Addr>,
}
