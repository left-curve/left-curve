use {
    crate::dex::PairId,
    grug::{Addr, Permission},
    serde::{Deserialize, Serialize},
    std::collections::BTreeMap,
};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct Permissions {
    pub swap: PairPermissions,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct PairPermissions {
    pub permissions: BTreeMap<PairId, Permission>,
    pub default_permission: Permission,
}

impl Default for Permissions {
    fn default() -> Self {
        Permissions {
            swap: PairPermissions {
                permissions: BTreeMap::new(),
                default_permission: Permission::Nobody,
            },
        }
    }
}

impl PairPermissions {
    pub fn has_permission(&self, pair_ids: &[PairId], sender: Addr) -> bool {
        pair_ids.iter().all(|pair_id| {
            self.permissions
                .get(pair_id)
                .unwrap_or(&self.default_permission) // Disallow for everyone by default.
                .has_permission(sender)
        })
    }
}
