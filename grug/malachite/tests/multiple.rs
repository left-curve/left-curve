use {
    crate::utils::load_config,
    dango_genesis::{GenesisCodes, GenesisOption},
    dango_testing::{Preset, TestOption, setup_suite_with_db_and_vm},
    dango_types::constants::usdc,
    grug::{Coins, Message, NonEmpty, ResultExt, Signer},
    grug_app::{NaiveProposalPreparer, NullIndexer},
    grug_db_memory::MemDb,
    grug_malachite::{MempoolMsg, PrivateKey, RawTx, Validator, ValidatorSet, spawn_actors},
    grug_vm_rust::RustVm,
    malachitebft_app::events::TxEvent,
    std::{sync::Arc, time::Duration},
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

    let mut actors = Vec::new();
    let mut accounts = Vec::new();

    for (i, name) in [
        (1, tracing::span!(tracing::Level::INFO, "node-1")),
        (2, tracing::span!(tracing::Level::INFO, "node-2")),
        (3, tracing::span!(tracing::Level::INFO, "node-3")),
    ] {
        let (suite, acc, ..) = setup_suite_with_db_and_vm(
            MemDb::new(),
            RustVm::new(),
            NaiveProposalPreparer,
            NullIndexer,
            codes,
            TestOption::preset_test(),
            GenesisOption::preset_test(),
        );

        let tx_event = TxEvent::new();

        let app = Arc::new(suite.app);

        let temp_dir = tempfile::tempdir().unwrap();

        let wal_path = temp_dir.path().join(format!("wal-{}", i));

        let actor = spawn_actors(
            Some(wal_path),
            load_config(format!("tests/nodes_config/node{}.toml", i), None).unwrap(),
            validator_set.clone(),
            None,
            tx_event,
            priv_keys[i - 1].clone(),
            app.clone(),
            name,
        )
        .await;

        actors.push(actor);
        accounts.push(acc);
    }

    tokio::time::sleep(Duration::from_secs(15)).await;

    // try broadcast a failing tx
    actors[1]
        .mempool
        .call(
            |reply| MempoolMsg::Add {
                tx: RawTx::from_bytes(vec![]),
                reply,
            },
            None,
        )
        .await
        .unwrap()
        .unwrap()
        .should_fail();

    tokio::time::sleep(Duration::from_secs(8)).await;

    // broadcast a tx
    let msg = Message::transfer(
        accounts[1].user2.address.into_inner(),
        Coins::one(usdc::DENOM.clone(), 100).unwrap(),
    )
    .unwrap();

    let tx = accounts
        .get_mut(1)
        .unwrap()
        .user1
        .sign_transaction(
            NonEmpty::new_unchecked(vec![msg]),
            &TestOption::preset_test().chain_id,
            1_000_000,
        )
        .unwrap();

    actors[1]
        .mempool
        .call(
            |reply| MempoolMsg::Add {
                tx: RawTx::from_tx(tx).unwrap(),
                reply,
            },
            None,
        )
        .await
        .unwrap()
        .unwrap()
        .should_succeed()
        .result
        .should_succeed();

    loop {}
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
