use {
    grug_types::{Batch, Hash256, Op},
    sha2::{Digest, Sha256},
};

pub fn batch_hash(batch: &Batch) -> Hash256 {
    let mut hasher = Sha256::new();
    for (k, op) in batch {
        hasher.update((k.len() as u16).to_be_bytes());
        hasher.update(k);
        if let Op::Insert(v) = op {
            hasher.update([1]);
            hasher.update((v.len() as u16).to_be_bytes());
            hasher.update(v);
        } else {
            hasher.update([0]);
        }
    }

    Hash256::from_inner(hasher.finalize().into())
}
