use {
    crate::utils::load_config,
    dango_genesis::{GenesisCodes, GenesisOption},
    dango_proposal_preparer::ProposalPreparer,
    dango_testing::{Preset, TestOption, setup_suite_with_db_and_vm},
    grug_app::NullIndexer,
    grug_db_memory::MemDb,
    grug_malachite::{PrivateKey, Validator, ValidatorSet, spawn_actors},
    grug_vm_rust::RustVm,
    malachitebft_app::events::TxEvent,
    std::{path::PathBuf, sync::Arc},
};

pub mod utils;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn multiple() {
    tracing::subscriber::set_global_default(
        tracing_subscriber::FmtSubscriber::builder()
            .with_file(false)
            .with_line_number(false)
            .with_target(false)
            .without_time()
            .with_max_level(tracing::Level::INFO)
            .finish(),
    )
    .unwrap();

    let (validator_set, priv_keys) = mock_validator_set();

    let codes = RustVm::genesis_codes();

    for (i, name) in [
        (1, tracing::span!(tracing::Level::INFO, "node-1")),
        (2, tracing::span!(tracing::Level::INFO, "node-2")),
        (3, tracing::span!(tracing::Level::INFO, "node-3")),
    ] {
        let (suite, ..) = setup_suite_with_db_and_vm(
            MemDb::new(),
            RustVm::new(),
            ProposalPreparer::new_with_cache(),
            NullIndexer,
            codes,
            TestOption::preset_test(),
            GenesisOption::preset_test(),
        );

        let tx_event = TxEvent::new();

        let app = Arc::new(suite.app);

        let _actors = spawn_actors(
            Some(PathBuf::from(format!("./tests/wals/wal-{}", i))),
            load_config(format!("tests/nodes_config/node{}.toml", i), None).unwrap(),
            validator_set.clone(),
            None,
            tx_event,
            priv_keys[i - 1].clone(),
            app.clone(),
            name,
        )
        .await;
    }

    loop {}

    // let (suite, ..) = setup_test(TestOption::preset_test());
}

fn mock_validator_set() -> (ValidatorSet, Vec<PrivateKey>) {
    let mut validators = Vec::new();
    let priv_keys = (1..=3)
        .into_iter()
        .map(|_| {
            let priv_key =
                PrivateKey::from_inner(k256::ecdsa::SigningKey::random(&mut rand::thread_rng()));

            let validator = Validator {
                address: priv_key.derive_address(),
                public_key: priv_key.public_key(),
                voting_power: 1,
            };

            validators.push(validator);
            priv_key
        })
        .collect();

    (ValidatorSet::new(validators), priv_keys)
}
