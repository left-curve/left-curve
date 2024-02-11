// only run this test if the "fuzzing" feature is enabled.
// this test takes very long to run, so we don't want it to be run by Github CI.
// we only run it manually:
// $ cargo test -p cw-jellyfish-merkle --features fuzzing --test fuzzing -- --nocapture
#![cfg(feature = "fuzzing")]

//! Our fuzzing strategy is as follows:
//!
//! - Write an initial batch of 100 random keys and values.
//!
//! - Write another 99 batches. Each batch consists of:
//!   - 50 inserts under existing keys
//!   - 30 inserts under new keys
//!   - 10 deletes of existing keys
//!   - 10 deletes of non-existing keys (should be no-op)
//!
//! - After the 100 batch writes, we do check each key that has ever been
//!   inserted or deleted, query the value and proof, check the values are
//!   correct and proofs are valid.
//!
//! Basically, we prove the following properties:
//!
//! - any KV pair that's in the tree can always be proven to exist against the
//!   root hash;
//! - any key that isn't in the tree can always be proven to not exist against
//!   the root hash.

use {
    anyhow::bail,
    cw_jmt::{verify_proof, MerkleTree},
    cw_std::{Batch, Hash, MockStorage, Op, Storage},
    rand::{rngs::StdRng, thread_rng, Rng, RngCore, SeedableRng},
};

const TREE:        MerkleTree  = MerkleTree::new_default();
const SEED:        Option<u64> = None;
const NUM_BATCHES: usize       = 100;

#[test]
fn fuzzing() -> anyhow::Result<()> {
    // if a seed is given, create a seeded RNG; otherwise created an unseeded one
    let mut rng: Box<dyn RngCore> = if let Some(seed) = SEED {
        Box::new(StdRng::seed_from_u64(seed))
    } else {
        Box::new(thread_rng())
    };
    let mut log = Batch::new();
    let mut store = MockStorage::new();

    let batch = generate_initial_batch(&mut rng);
    TREE.apply(&mut store, batch.clone()).unwrap();
    log.extend(batch);
    check(&store, &log, 1)?;

    for i in 2..=NUM_BATCHES {
        let batch = generate_subsequent_batch(&log, &mut rng);
        TREE.apply(&mut store, batch.clone()).unwrap();
        log.extend(batch);
        check(&store, &log, i)?;
    }

    Ok(())
}

fn random_hash<R: Rng>(rng: &mut R) -> Hash {
    let mut slice = [0u8; Hash::LENGTH];
    rng.fill(&mut slice);
    Hash::from_slice(slice)
}

fn random_item_from_log<'a, R: Rng>(
    log: &'a Batch<Hash, Hash>,
    rng: &mut R,
) -> (&'a Hash, &'a Op<Hash>) {
    log.iter().nth(rng.gen_range(0..log.len())).unwrap()
}

fn generate_initial_batch<R: Rng>(rng: &mut R) -> Vec<(Hash, Op<Hash>)> {
    let mut batch = Batch::new();
    for _ in 0..100 {
        batch.insert(random_hash(rng), Op::Insert(random_hash(rng)));
    }
    batch.into_iter().collect()
}

fn generate_subsequent_batch<R: Rng>(log: &Batch<Hash, Hash>, rng: &mut R) -> Vec<(Hash, Op<Hash>)> {
    let mut batch = Batch::new();
    // 50 inserts under existing keys
    for _ in 0..50 {
        loop {
            let (key, op) = random_item_from_log(log, rng);
            if let Op::Insert(_) = op {
                batch.insert(key.clone(), Op::Insert(random_hash(rng)));
                break;
            }
        }
    }
    // 10 deletes under existing keys
    for _ in 0..10 {
        loop {
            let (key, op) = random_item_from_log(log, rng);
            if let Op::Insert(_) = op {
                batch.insert(key.clone(), Op::Delete);
                break;
            }
        }
    }
    // 30 inserts under possibly new keys
    for _ in 0..30 {
        batch.insert(random_hash(rng), Op::Insert(random_hash(rng)));
    }
    // 10 deletes under possibly new keys
    for _ in 0..10 {
        batch.insert(random_hash(rng), Op::Delete);
    }
    batch.into_iter().collect()
}

fn check(store: &dyn Storage, log: &Batch<Hash, Hash>, i: usize) -> anyhow::Result<()> {
    let Some(root_hash) = TREE.root_hash(store, None)? else {
        bail!("batch {i}: root hash is empty");
    };

    println!("batch {i}: root = {root_hash}");

    for (key_hash, op) in log {
        let proof = TREE.prove(store, key_hash, None)?;
        verify_proof(&root_hash, key_hash, op.as_ref().into_option(), &proof)?;
    }

    Ok(())
}
