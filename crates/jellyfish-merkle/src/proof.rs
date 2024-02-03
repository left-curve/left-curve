use cw_std::{Hash, StdResult};

pub struct Proof {
    // TODO
}

pub fn verify_membership(
    _root_hash:  &Hash,
    _key_hash:   &Hash,
    _value_hash: &Hash,
    _proof:      &Proof,
) -> StdResult<()> {
    todo!()
}

pub fn verify_non_membership(
    _root_hash: &Hash,
    _key_hash:  &Hash,
    _proof:     &Proof,
) -> StdResult<()> {
    todo!()
}
