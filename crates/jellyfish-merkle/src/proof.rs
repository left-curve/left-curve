use cw_std::{Hash, StdResult};

pub struct Proof {
    // TODO
}

pub fn verify_membership(
    root_hash:  &Hash,
    key_hash:   &Hash,
    value_hash: &Hash,
    proof:      &Proof,
) -> StdResult<()> {
    todo!()
}

pub fn verify_non_membership(root_hash: &Hash, key_hash: &Hash, proof: &Proof) -> StdResult<()> {
    todo!()
}
