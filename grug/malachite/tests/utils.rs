use {
    dango_genesis::{GenesisCodes, GenesisOption},
    dango_testing::{Preset, TestAccounts, TestOption, setup_suite_with_db_and_vm},
    grug_app::{AppError, Db, Indexer, NaiveProposalPreparer, NullIndexer, ProposalPreparer, Vm},
    grug_malachite::{Actors, ActorsConfig, PrivateKey, Validator, ValidatorSet, spawn_actors},
    grug_vm_rust::RustVm,
    std::{path::Path, sync::Arc},
    tracing::Span,
};

pub fn setup_tracing() {
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
}

pub fn load_config(path: impl AsRef<Path>, prefix: Option<&str>) -> anyhow::Result<ActorsConfig> {
    ::config::Config::builder()
        .add_source(::config::File::from(path.as_ref()))
        .add_source(
            ::config::Environment::with_prefix(prefix.unwrap_or("MALACHITE")).separator("__"),
        )
        .build()?
        .try_deserialize()
        .map_err(Into::into)
}

pub async fn launch_nodes<const N: usize, DB>(
    spans: [(Span, DB); N],
) -> ([Actors; N], [TestAccounts; N])
where
    DB: Db + Send + Sync + 'static,
    DB::Error: std::fmt::Debug,
    AppError: From<DB::Error>
        + From<<RustVm as Vm>::Error>
        + From<<NaiveProposalPreparer as ProposalPreparer>::Error>
        + From<<NullIndexer as Indexer>::Error>,
{
    let (validator_set, priv_keys) = mock_validator_set::<N>();

    let mut actors = Vec::new();
    let mut accounts = Vec::new();
    let codes = RustVm::genesis_codes();

    for (i, (span, db)) in spans.into_iter().enumerate() {
        let (suite, acc, ..) = setup_suite_with_db_and_vm(
            db,
            RustVm::new(),
            NaiveProposalPreparer,
            NullIndexer,
            codes,
            TestOption::preset_test(),
            GenesisOption::preset_test(),
        );

        let app = Arc::new(suite.app);

        let temp_dir = tempfile::tempdir().unwrap();

        let wal_path = temp_dir.path().join(format!("wal-{}", i));

        let actor = spawn_actors(
            Some(wal_path),
            // load_config(format!("tests/nodes_config/node{}.toml", i), None).unwrap(),
            create_config(i, N),
            validator_set.clone(),
            priv_keys[i].clone(),
            app.clone(),
            None,
            Some(span),
        )
        .await;

        actors.push(actor);
        accounts.push(acc);
    }

    (actors.try_into().unwrap(), accounts.try_into().unwrap())
}

fn mock_validator_set<const N: usize>() -> (ValidatorSet, [PrivateKey; N]) {
    let mut validators = Vec::new();
    let priv_keys = (0..N)
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
        .collect::<Vec<_>>();

    (ValidatorSet::new(validators), priv_keys.try_into().unwrap())
}

fn create_config(i: usize, max: usize) -> ActorsConfig {
    let mut cfg = ActorsConfig::default();

    cfg.mempool.p2p.listen_addr = format!("/ip4/127.0.0.1/tcp/{}", 28000 + i).parse().unwrap();
    cfg.consensus.p2p.listen_addr = format!("/ip4/127.0.0.1/tcp/{}", 27000 + i).parse().unwrap();
    cfg.metrics.listen_addr = format!("127.0.0.1:{}", 29000 + i).parse().unwrap();

    for j in 0..max {
        if i == j {
            continue;
        }
        cfg.consensus
            .p2p
            .persistent_peers
            .push(format!("/ip4/127.0.0.1/tcp/{}", 27000 + j).parse().unwrap());

        cfg.mempool
            .p2p
            .persistent_peers
            .push(format!("/ip4/127.0.0.1/tcp/{}", 28000 + j).parse().unwrap());
    }

    cfg
}
