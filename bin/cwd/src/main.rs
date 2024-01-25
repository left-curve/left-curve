use {
    clap::Parser, cw_app::App, cw_db::MockStorage, std::path::PathBuf,
    tracing::metadata::LevelFilter,
};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// ABCI listening address
    #[arg(long, default_value = "127.0.0.1:26658")]
    pub addr: String,

    /// Directory for the physical database
    #[arg(long, default_value = "~/.cwd")]
    pub db_dir: PathBuf,

    /// Use a in-memory mock storage instead of a persisted physical database
    #[arg(long)]
    pub mock: bool,

    /// Buffer size for reading chunks of incoming data from client
    #[arg(long, default_value = "1048576")]
    pub read_buf_size: usize,

    /// Logging verbosity: error|warn|info|debug|trace
    #[arg(long, default_value = "info")]
    pub tracing_level: LevelFilter,
}

fn main() -> anyhow::Result<()> {
    // parse command line input
    let cli = Cli::parse();

    // set tracing level
    tracing_subscriber::fmt().with_max_level(cli.tracing_level).init();

    // create DB backend
    let store = if cli.mock {
        MockStorage::new()
    } else {
        todo!("persisted DB backend isn't implemented yet")
    };

    // start the ABCI server
    App::new(store).start_abci_server(cli.read_buf_size, cli.addr).map_err(Into::into)
}
