use {
    crate::{DbResult, DiskDb, MERKLE_TREE, cf_preimages, new_read_options},
    grug_app::{Db, IbcDb},
    grug_jmt::ICS23_PROOF_SPEC,
    grug_types::{HashExt, Storage},
    ics23::{
        CommitmentProof, ExistenceProof, NonExistenceProof,
        commitment_proof::Proof as CommitmentProofInner,
    },
    rocksdb::{Direction, IteratorMode},
};

impl IbcDb for DiskDb {
    fn ics23_prove(
        &self,
        key: Vec<u8>,
        version: Option<u64>,
    ) -> Result<CommitmentProof, Self::Error> {
        let version = version.unwrap_or_else(|| self.latest_version().unwrap_or(0));
        let state_storage = self.state_storage(Some(version))?;
        let state_commitment = self.state_commitment();

        let generate_existence_proof = |key: Vec<u8>, value| -> DbResult<_> {
            let key_hash = key.hash256();
            let path = MERKLE_TREE.ics23_prove_existence(&state_commitment, version, key_hash)?;

            Ok(ExistenceProof {
                key,
                value,
                leaf: ICS23_PROOF_SPEC.leaf_spec.clone(),
                path,
            })
        };

        let proof = match state_storage.read(&key) {
            // Value is found. Generate an ICS-23 existence proof.
            Some(value) => CommitmentProofInner::Exist(generate_existence_proof(key, value)?),
            // Value is not found.
            //
            // Here, unlike Diem or Penumbra's implementation, which walks the
            // tree to find the left and right neighbors, we use an approach
            // similar to SeiDB's:
            // https://github.com/sei-protocol/sei-db/blob/v0.0.43/sc/memiavl/proof.go#L41-L76
            //
            // We simply look up the state storage to find the left and right
            // neighbors, and generate existence proof of them.
            None => {
                let cf = cf_preimages(&self.inner.db);
                let key_hash = key.hash256();

                let opts = new_read_options(Some(version), None, None);
                let mode = IteratorMode::From(&key_hash, Direction::Reverse);
                let left = self
                    .inner
                    .db
                    .iterator_cf_opt(&cf, opts, mode)
                    .next()
                    .map(|res| {
                        let (_, key) = res?;
                        let value = state_storage.read(&key).unwrap();
                        generate_existence_proof(key.to_vec(), value)
                    })
                    .transpose()?;

                let opts = new_read_options(Some(version), None, None);
                let mode = IteratorMode::From(&key_hash, Direction::Forward);
                let right = self
                    .inner
                    .db
                    .iterator_cf_opt(&cf, opts, mode)
                    .next()
                    .map(|res| {
                        let (_, key) = res?;
                        let value = state_storage.read(&key).unwrap();
                        generate_existence_proof(key.to_vec(), value)
                    })
                    .transpose()?;

                CommitmentProofInner::Nonexist(NonExistenceProof { key, left, right })
            },
        };

        Ok(CommitmentProof { proof: Some(proof) })
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        grug_types::{Batch, Op},
        ics23::HostFunctionsManager,
        proptest::prelude::*,
        std::collections::BTreeMap,
        temp_rocksdb::TempDataDir,
    };

    #[test]
    fn ics23_prove_works() {
        let path = TempDataDir::new("_grug_disk_db_ics23_proving_works");
        let db = DiskDb::open(&path).unwrap();

        // Same test data as used in JMT crate.
        let (_, maybe_root) = db
            .flush_and_commit(Batch::from([
                (b"r".to_vec(), Op::Insert(b"foo".to_vec())),
                (b"m".to_vec(), Op::Insert(b"bar".to_vec())),
                (b"L".to_vec(), Op::Insert(b"fuzz".to_vec())),
                (b"a".to_vec(), Op::Insert(b"buzz".to_vec())),
            ]))
            .unwrap();
        let root = maybe_root.unwrap().to_vec();

        // Prove existing keys, and verify those proofs.
        for (key, value) in [("r", "foo"), ("m", "bar"), ("L", "fuzz"), ("a", "buzz")] {
            let proof = db.ics23_prove(key.as_bytes().to_vec(), None).unwrap();
            assert!(
                ics23::verify_membership::<HostFunctionsManager>(
                    &proof,
                    &ICS23_PROOF_SPEC,
                    &root,
                    key.as_bytes(),
                    value.as_bytes(),
                ),
                "inclusion verification failed for key `{key}` and value `{value}`"
            );
        }

        // Prove non-existing keys, and verify those proofs.
        for key in ["b", "o"] {
            let proof = db.ics23_prove(key.as_bytes().to_vec(), None).unwrap();
            assert!(
                ics23::verify_non_membership::<HostFunctionsManager>(
                    &proof,
                    &ICS23_PROOF_SPEC,
                    &root,
                    key.as_bytes()
                ),
                "exclusion verification failed for key `{key}`"
            );
        }
    }

    /// Testing a coding mistake found in the Zellic audit (finding 3.1).
    ///
    /// If a batch contains deletions, we forgot to also delete the keys from
    /// the preimages map.
    ///
    /// In this example,
    ///
    /// - We build the tree at version 0 with two keys: "b" and "a".
    /// - Then, at version 1, we delete "a".
    /// - Then, we prove the non-existence of "L" at version 1.
    ///
    /// Hashes:
    ///
    /// - sha256("m") = 0110...
    /// - sha256("L") = 0111...
    /// - sha256("a") = 1100...
    ///
    /// Their relation is:
    ///
    /// > "m" < "L" < "a"
    ///
    /// At version 1, we expect the ICS-23 proof to contain a left neighbor ("m")
    /// and no right neighbor.
    ///
    /// However, since we don't to delete the preimage of "a", the function
    /// incorrectly thinks "a" exists as the right neighbor.
    ///
    /// When loading the key from state storage, it panics.
    #[test]
    fn ics23_prove_after_deletion() {
        let path = TempDataDir::new("__grug_disk_db_ics23_prove_after_deletion");
        let db = DiskDb::open(&path).unwrap();

        // Apply batch at version 0.
        let _ = db
            .flush_and_commit(Batch::from([
                (b"b".to_vec(), Op::Insert(b"buzz".to_vec())),
                (b"a".to_vec(), Op::Insert(b"fuzz".to_vec())),
            ]))
            .unwrap();

        // Apply batch at version 1.
        let (version, maybe_root) = db
            .flush_and_commit(Batch::from([(b"a".to_vec(), Op::Delete)]))
            .unwrap();

        // Generate and verify non-existence proof of "L" at version 1.
        let key = b"L".to_vec();
        let proof = db.ics23_prove(key.clone(), Some(version)).unwrap();
        assert!(ics23::verify_non_membership::<HostFunctionsManager>(
            &proof,
            &ICS23_PROOF_SPEC,
            &maybe_root.unwrap().to_vec(),
            &key,
        ));
    }

    proptest! {
        /// Apply three batches as follows:
        ///
        /// 1. Insertions only.
        /// 2. Deletions only.
        /// 3. Both insertions and deletions.
        ///
        /// After each batch, generate and verify ICS-23 proofs.
        #[test]
        fn proptest_ics23_prove_works(
            (inserts1, deletes1) in prop::collection::hash_map("[a-z]{1,10}", "[a-z]{1,10}", 1..100).prop_flat_map(|kvs| {
                let len = kvs.len();
                (Just(kvs), prop::collection::vec(any::<prop::sample::Selector>(), 0..len))
            }),
            inserts2 in prop::collection::hash_map("[a-z]{1,10}", "[a-z]{1,10}", 1..100),
            deletes2 in prop::collection::vec(any::<prop::sample::Selector>(), 0..50)
        ) {
            let path = TempDataDir::new("_grug_disk_db_ics23_proving_works");
            let db = DiskDb::open(&path).unwrap();
            let mut state = BTreeMap::new();

            // --------------------------- version 0 ---------------------------

            let batch0 = inserts1
                .into_iter()
                .map(|(k, v)| (k.into_bytes(), Op::Insert(v.into_bytes())))
                .collect::<Batch>();
            let (version0, maybe_root) = db.flush_and_commit(batch0.clone()).unwrap();
            let root0 = maybe_root.unwrap().to_vec();
            assert_eq!(version0, 0);

            // Update state mirroring version 0.
            for (k, v) in &batch0 {
                state.insert(k.clone(), v.clone().into_option().unwrap());
            }

            // Generate and verify membership proofs for each key in the state of
            // version 0.
            for (k, v) in &state {
                let proof = db.ics23_prove(k.clone(), Some(version0)).unwrap();
                assert!(ics23::verify_membership::<HostFunctionsManager>(
                    &proof,
                    &ICS23_PROOF_SPEC,
                    &root0,
                    k,
                    v,
                ));
            }

            // --------------------------- version 1 ---------------------------

            let batch1 = deletes1
                .into_iter()
                .map(|s| {
                    let (k, _) = s.select(&batch0);
                    (k.clone(), Op::Delete)
                })
                .collect::<Batch>();
            let (version1, maybe_root) = db.flush_and_commit(batch1.clone()).unwrap();
            let root1 = maybe_root.unwrap().to_vec();
            assert_eq!(version1, 1);

            // For each key in this batch,
            // - generate and verify membership proof at verson 0, when the key
            //   still existsed in the state;
            // - generate and verify non-membership proof at version 1, after the
            //   key has been deleted.
            // Also update the state.
            for k in batch1.keys() {
                // Prove in version 0
                let proof = db.ics23_prove(k.clone(), Some(version0)).unwrap();
                assert!(ics23::verify_membership::<HostFunctionsManager>(
                    &proof,
                    &ICS23_PROOF_SPEC,
                    &root0,
                    k,
                    state.get(k).unwrap(),
                ));

                // Prove in version 1
                let proof = db.ics23_prove(k.clone(), Some(version1)).unwrap();
                assert!(ics23::verify_non_membership::<HostFunctionsManager>(
                    &proof,
                    &ICS23_PROOF_SPEC,
                    &root1,
                    k,
                ));

                // Update the state mirroring version 1.
                state.remove(k);
            }

            // --------------------------- version 2 ---------------------------

            let deletes2 = deletes2
                .into_iter()
                .map(|s| {
                    let (k, _) = s.select(&batch0);
                    (k.clone(), Op::Delete)
                })
                .collect::<Batch>();
            let batch2 = inserts2
                .into_iter()
                .map(|(k, v)| (k.into_bytes(), Op::Insert(v.into_bytes())))
                .chain(deletes2.clone())
                .collect::<Batch>();
            let (version2, maybe_root) = db.flush_and_commit(batch2.clone()).unwrap();
            let root2 = maybe_root.unwrap().to_vec();
            assert_eq!(version2, 2);

            // Update the state mirroring version 2.
            for (k, op) in batch2 {
                match op {
                    Op::Insert(v) => {
                        state.insert(k.clone(), v);
                    },
                    Op::Delete => {
                        state.remove(&k);
                    },
                }
            }

            // Generate and verify proofs at version 2.
            for (k, v) in &state {
                let proof = db.ics23_prove(k.clone(), Some(version2)).unwrap();
                assert!(ics23::verify_membership::<HostFunctionsManager>(
                    &proof,
                    &ICS23_PROOF_SPEC,
                    &root2,
                    k,
                    v,
                ));
            }
            for k in deletes2.keys() {
                let proof = db.ics23_prove(k.clone(), Some(version2)).unwrap();
                assert!(ics23::verify_non_membership::<HostFunctionsManager>(
                    &proof,
                    &ICS23_PROOF_SPEC,
                    &root2,
                    k,
                ));
            }
        }
    }
}
