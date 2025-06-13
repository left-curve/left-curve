use {
    crate::utils::load_config,
    dango_testing::{Preset, TestOption, setup_test},
    grug::setup_tracing_subscriber,
    grug_malachite::{PrivateKey, Validator, ValidatorSet, spawn_actors},
    std::{path::PathBuf, sync::Arc},
};

pub mod utils;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn single() {
    setup_tracing_subscriber(tracing::Level::DEBUG);

    let (suite, ..) = setup_test(TestOption::preset_test());

    let (validator_set, priv_key) = mock_validator_set();

    let app = Arc::new(suite.app);

    let _actors = spawn_actors(
        Some(PathBuf::from("./tests/wals/wal-single")),
        load_config("tests/nodes_config/node1.toml", None).unwrap(),
        validator_set,
        priv_key,
        app.clone(),
        None,
        Some(tracing::span!(tracing::Level::INFO, "consensus")),
    )
    .await;

    loop {}
}

fn mock_validator_set() -> (ValidatorSet, PrivateKey) {
    let priv_key = PrivateKey::from_inner(k256::ecdsa::SigningKey::random(&mut rand::thread_rng()));

    let validator = Validator {
        address: priv_key.derive_address(),
        public_key: priv_key.public_key(),
        voting_power: 1,
    };

    (ValidatorSet::new(vec![validator]), priv_key)
}
