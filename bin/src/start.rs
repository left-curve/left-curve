use {
    clap::Parser, grug_app::App, grug_db_disk::DiskDb, grug_types::Size, grug_vm_wasm::WasmVm,
    std::path::PathBuf,
};

#[derive(Parser)]
pub struct StartCmd {
    /// Tendermint ABCI listening address
    #[arg(long, default_value = "127.0.0.1:26658")]
    abci_addr: String,

    /// Size of the read buffer for each incoming connection to the ABCI server, in bytes
    #[arg(long, default_value = "1048576")]
    read_buf_size: usize,

    /// Size of the wasm module cache, in megabytes
    #[arg(long, default_value = "1000")]
    wasm_cache_size: usize,
}

impl StartCmd {
    pub async fn run(self, data_dir: PathBuf) -> anyhow::Result<()> {
        // Create DB backend
        let db = DiskDb::open(data_dir)?;

        // Create the app
        let app = App::<DiskDb, WasmVm>::new(db, Size::mega(1000));

        // Start the ABCI server
        Ok(app.start_abci_server(self.read_buf_size, self.abci_addr)?)
    }
}
