use {
    crate::{
        constants::{dango, usdc},
        dex::PairId,
    },
    grug::{Addr, Permission},
    std::collections::BTreeMap,
};

#[grug::derive(Serde)]
pub struct Permissions {
    pub swap: PairPermissions,
}

#[grug::derive(Serde)]
pub struct PairPermissions {
    pub permissions: Vec<(PairId, Permission)>,
    pub default_permission: Permission,
}

impl Default for Permissions {
    fn default() -> Self {
        Permissions {
            swap: PairPermissions {
                permissions: vec![(
                    PairId {
                        base_denom: dango::DENOM.clone(),
                        quote_denom: usdc::DENOM.clone(),
                    },
                    Permission::Nobody,
                )],
                default_permission: Permission::Everybody,
            },
        }
    }
}

impl PairPermissions {
    pub fn has_permission(&self, pair_ids: &[PairId], sender: Addr) -> bool {
        pair_ids.iter().all(|pair_id| {
            self.permissions.clone().into_iter().collect::<BTreeMap<PairId, Permission>>()
                .get(pair_id)
                .unwrap_or(&self.default_permission) // Disallow for everyone by default.
                .has_permission(sender)
        })
    }
}
