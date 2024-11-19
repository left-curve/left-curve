use {
    clap::Parser,
    dango_app::ProposalPreparer as DangoProposalPreparer,
    grug_app::{App, AppError, Db, ProposalPreparer, Vm},
    grug_db_disk::DiskDb,
    grug_vm_wasm::WasmVm,
    std::{path::PathBuf, time},
    tower::ServiceBuilder,
    tower_abci::v038::{split, Server},
};

#[derive(Parser)]
pub struct StartCmd {
    /// Tendermint ABCI listening address
    #[arg(long, default_value = "127.0.0.1:26658")]
    abci_addr: String,

    /// Size of the read buffer for each incoming connection to the ABCI server, in bytes
    #[arg(long, default_value = "1048576")]
    read_buf_size: usize,

    /// Capacity of the wasm module cache; zero means do not use a cache
    #[arg(long, default_value = "1000")]
    wasm_cache_capacity: usize,

    /// Gas limit when serving query requests [default: u64::MAX]
    #[arg(long)]
    query_gas_limit: Option<u64>,

    /// Use tower over tendermint_abci
    #[arg(long)]
    tower: bool,
}

impl StartCmd {
    pub async fn run(self, data_dir: PathBuf) -> anyhow::Result<()> {
        let db = DiskDb::open(data_dir)?;
        let vm = WasmVm::new(self.wasm_cache_capacity);

        let app = App::new(
            db,
            vm,
            DangoProposalPreparer::new(),
            self.query_gas_limit.unwrap_or(u64::MAX),
        );

        if self.tower {
            start_tower(app, self.abci_addr).await
        } else {
            Ok(app.start_abci_server(self.read_buf_size, self.abci_addr)?)
        }
    }
}

async fn start_tower<DB, VM, PP>(app: App<DB, VM, PP>, path: String) -> anyhow::Result<()>
where
    DB: Db + Send + 'static,
    VM: Vm + Clone + Send + 'static,
    PP: ProposalPreparer + Send + 'static,
    AppError: From<DB::Error> + From<VM::Error> + From<PP::Error>,
{
    let (consensus, mempool, snapshot, info) = split::service(app, 1);

    let server_builder = Server::builder()
        .consensus(consensus)
        .snapshot(snapshot)
        .mempool(
            ServiceBuilder::new()
                .load_shed()
                .buffer(100)
                .service(mempool),
        )
        .info(
            ServiceBuilder::new()
                .load_shed()
                .buffer(100)
                .rate_limit(50, time::Duration::from_secs(1))
                .service(info),
        );

    let server = server_builder.finish().unwrap();

    server.listen_tcp(path).await.unwrap();
    Ok(())
}
