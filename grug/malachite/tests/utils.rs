use {
    dango_genesis::{GenesisCodes, GenesisOption},
    dango_testing::{Preset, TestAccounts, TestOption, setup_suite_with_db_and_vm},
    grug_app::{
        App, AppError, Db, Indexer, NaiveProposalPreparer, NullIndexer, ProposalPreparer, Vm,
    },
    grug_db_disk_lite::DiskDbLite,
    grug_malachite::{
        Actors, ActorsConfig, HostApp, MempoolApp, PrivateKey, Validator, ValidatorSet,
        spawn_actors,
    },
    grug_vm_rust::RustVm,
    std::{
        path::{Path, PathBuf},
        sync::Arc,
    },
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

pub struct Nodes<const N: usize, DB, VM, PP, ID> {
    pub actors: [Actors; N],
    pub accounts: [TestAccounts; N],
    validator_set: ValidatorSet,
    priv_keys: [PrivateKey; N],
    wal_paths: [PathBuf; N],
    apps: [Arc<App<DB, VM, PP, ID>>; N],
    spans: [Span; N],
}

impl<const N: usize, DB, VM, PP, ID> Nodes<N, DB, VM, PP, ID> {
    pub fn stop_actor(&self, i: usize) {
        self.actors[i].node.stop(None);
    }
}

impl<const N: usize, DB, VM, PP, ID> Nodes<N, DB, VM, PP, ID>
where
    VM: Vm + Clone + Send + Sync,
    PP: ProposalPreparer,
    ID: Indexer,
    DB: Db + RelaunchableDb,
    DB::Error: std::fmt::Debug,
    AppError: From<DB::Error> + From<VM::Error> + From<PP::Error> + From<ID::Error>,
    App<DB, VM, PP, ID>: MempoolApp,
    App<DB, VM, PP, ID>: HostApp,
{
    pub async fn relaunch_actor(&mut self, i: usize) {
        let actor = spawn_actors(
            Some(self.wal_paths[i].clone()),
            create_config(i, N),
            self.validator_set.clone(),
            self.priv_keys[i].clone(),
            self.apps[i].clone(),
            None,
            Some(self.spans[i].clone()),
        )
        .await;

        self.actors[i] = actor;
    }
}

pub trait RelaunchableDb: Db {}

impl RelaunchableDb for DiskDbLite {}

pub async fn launch_nodes<const N: usize, DB>(
    spans: [(Span, DB); N],
) -> Nodes<N, DB, RustVm, NaiveProposalPreparer, NullIndexer>
where
    DB: Db + Send + Sync + 'static,
    DB::Error: std::fmt::Debug,
    AppError: From<DB::Error>
        + From<<RustVm as Vm>::Error>
        + From<<NaiveProposalPreparer as ProposalPreparer>::Error>
        + From<<NullIndexer as Indexer>::Error>,
{
    let (validator_set, priv_keys) = mock_validator_set::<N>();

    let mut actors = Vec::with_capacity(N);
    let mut accounts = Vec::with_capacity(N);
    let codes = RustVm::genesis_codes();
    let mut wal_paths = Vec::with_capacity(N);
    let mut apps = Vec::with_capacity(N);
    let mut spanss = Vec::with_capacity(N);

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

        let wal_path = temp_dir(format!("wal-{}", i));

        wal_paths.push(wal_path.clone());

        let actor = spawn_actors(
            Some(wal_path),
            // load_config(format!("tests/nodes_config/node{}.toml", i), None).unwrap(),
            create_config(i, N),
            validator_set.clone(),
            priv_keys[i].clone(),
            app.clone(),
            None,
            Some(span.clone()),
        )
        .await;

        actors.push(actor);
        accounts.push(acc);
        apps.push(app);
        spanss.push(span);
    }

    Nodes {
        actors: actors.try_into().unwrap(),
        accounts: accounts.try_into().unwrap(),
        validator_set,
        priv_keys,
        wal_paths: wal_paths.try_into().unwrap(),
        apps: apps
            .try_into()
            .map_err(|_| anyhow::anyhow!("Failed to convert apps to array"))
            .unwrap(),
        spans: spanss.try_into().unwrap(),
    }
}

pub fn temp_dir<P>(prefix: P) -> PathBuf
where
    P: AsRef<Path>,
{
    tempfile::tempdir().unwrap().path().join(prefix.as_ref())
}

fn mock_validator_set<const N: usize>() -> (ValidatorSet, [PrivateKey; N]) {
    let mut validators = Vec::with_capacity(N);
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
