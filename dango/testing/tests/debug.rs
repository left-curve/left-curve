use {
    dango_genesis::{GenesisCodes, GenesisOption},
    dango_testing::{Preset, TestOption, setup_suite_with_db_and_vm},
    grug_app::{Db, NaiveProposalPreparer, NullIndexer},
    grug_db_disk_lite::DiskDbLite,
    grug_vm_rust::RustVm,
};

pub struct WrapperDb<DB> {
    db: DB,
}

impl<DB> WrapperDb<DB> {
    pub fn new(db: DB) -> Self {
        Self { db }
    }
}

impl<DB> Db for WrapperDb<DB>
where
    DB: Db,
{
    type Error = DB::Error;
    type Proof = DB::Proof;
    type StateCommitment = DB::StateCommitment;
    type StateStorage = DB::StateStorage;

    fn state_commitment(&self) -> Self::StateCommitment {
        self.db.state_commitment()
    }

    fn state_storage(&self, version: Option<u64>) -> Result<Self::StateStorage, Self::Error> {
        self.db.state_storage(version)
    }

    fn latest_version(&self) -> Option<u64> {
        self.db.latest_version()
    }

    fn root_hash(&self, version: Option<u64>) -> Result<Option<grug::Hash256>, Self::Error> {
        self.db.root_hash(version)
    }

    fn prove(&self, key: &[u8], version: Option<u64>) -> Result<Self::Proof, Self::Error> {
        self.db.prove(key, version)
    }

    fn flush_but_not_commit(
        &self,
        batch: grug::Batch,
    ) -> Result<(u64, Option<grug::Hash256>), Self::Error> {
        self.db.flush_but_not_commit(batch)
    }

    fn commit(&self) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[test]
fn testnet_debug() {
    let db = WrapperDb::new(DiskDbLite::open("../../testnet_data").unwrap());

    let (mut suite, ..) = setup_suite_with_db_and_vm(
        db,
        RustVm::new(),
        NaiveProposalPreparer,
        NullIndexer,
        RustVm::genesis_codes(),
        TestOption::default(),
        GenesisOption::preset_test(),
    );

    let chain_id = suite.query_status().unwrap();
    suite.chain_id = chain_id.chain_id;
    println!("{}", chain_id.last_finalized_block.height);
    suite.block = chain_id.last_finalized_block;

    let res = suite.make_empty_block().block_outcome;
}
