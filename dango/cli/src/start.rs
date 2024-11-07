use {
    anyhow::bail,
    clap::Parser,
    dango_app::ProposalPreparer,
    grug_app::{App, NaiveProposalPreparer},
    grug_db_disk::DiskDb,
    grug_types::Addr,
    grug_vm_wasm::WasmVm,
    std::{path::PathBuf, str::FromStr},
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

    /// Pyth feeder chain_id
    #[arg(long)]
    chain_id: Option<String>,

    /// Pyth feeder username
    #[arg(long)]
    feeder_username: Option<String>,

    /// Pyth feeder address hex string
    #[arg(long)]
    feeder_addr: Option<String>,

    /// Pyth feeder secret key hex string
    #[arg(long)]
    feeder_sk: Option<String>,
}

impl StartCmd {
    pub async fn run(self, data_dir: PathBuf) -> anyhow::Result<()> {
        let db = DiskDb::open(data_dir)?;
        let vm = WasmVm::new(self.wasm_cache_capacity);

        match (
            &self.chain_id,
            &self.feeder_username,
            &self.feeder_addr,
            &self.feeder_sk,
        ) {
            (Some(chain_id), Some(feeder_username), Some(feeder_addr), Some(feeder_sk)) => {
                let pp = ProposalPreparer::new(
                    chain_id.clone(),
                    Addr::from_str(feeder_addr)?,
                    &hex::decode(feeder_sk)?,
                    feeder_username.clone(),
                )?;

                let app = App::new(db, vm, pp, self.query_gas_limit.unwrap_or(u64::MAX));

                Ok(app.start_abci_server(self.read_buf_size, self.abci_addr)?)
            },
            (None, None, None, None) => {
                let app = App::new(
                    db,
                    vm,
                    NaiveProposalPreparer,
                    self.query_gas_limit.unwrap_or(u64::MAX),
                );

                Ok(app.start_abci_server(self.read_buf_size, self.abci_addr)?)
            },
            _ => bail!(
                "Not all pyth feeder parameters are provided:
                chain_id: {:?},
                feeder_username: {:?},
                feeder_addr: {:?},
                feeder_sk: {:?}",
                self.chain_id,
                self.feeder_username,
                self.feeder_addr,
                self.feeder_sk
            ),
        }
    }
}
