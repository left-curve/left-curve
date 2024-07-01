use {clap::Parser, grug_app::App, grug_db_disk::DiskDb, grug_vm_wasm::WasmVm, std::path::PathBuf};

#[derive(Parser)]
pub struct StartCmd {
    /// Tendermint ABCI listening address
    #[arg(long, default_value = "127.0.0.1:26658")]
    abci_addr: String,

    /// Buffer size for reading chunks of incoming data from client
    #[arg(long, default_value = "1048576")]
    read_buf_size: usize,
}

impl StartCmd {
    pub async fn run(self, data_dir: PathBuf) -> anyhow::Result<()> {
        let db = DiskDb::open(data_dir)?;
        let vm = WasmVm::new();
        let app = App::new(db, vm);

        Ok(app.start_abci_server(self.read_buf_size, self.abci_addr)?)
    }
}
