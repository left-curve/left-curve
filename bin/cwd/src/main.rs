use {
    anyhow::anyhow, clap::Parser, cw_app::App, cw_db::BaseStore, home::home_dir,
    std::path::PathBuf, tracing::metadata::LevelFilter,
};

// relative to user home directory (~)
const DEFAULT_DATA_DIR: &str = ".cwd";

#[derive(Parser)]
#[command(author, version, about, next_display_order = None)]
struct Cli {
    /// ABCI listening address
    #[arg(long, default_value = "127.0.0.1:26658")]
    pub addr: String,

    /// Directory for the physical database
    #[arg(long)]
    pub db_dir: Option<PathBuf>,

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

    // find DB directory
    let data_dir = if let Some(dir) = cli.db_dir {
        dir
    } else {
        let home_dir = home_dir().ok_or(anyhow!("failed to find home directory"))?;
        home_dir.join(DEFAULT_DATA_DIR)
    };

    // create DB backend
    let store = BaseStore::open(data_dir)?;

    // start the ABCI server
    App::new(store).start_abci_server(cli.read_buf_size, cli.addr).map_err(Into::into)
}
