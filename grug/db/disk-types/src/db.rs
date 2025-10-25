use {
    rocksdb::{DBWithThreadMode, MultiThreaded, Options},
    std::path::Path,
};

pub type MultiThreadedDb = DBWithThreadMode<MultiThreaded>;

pub fn open_db<P, I, N>(data_dir: P, cfs: I) -> Result<MultiThreadedDb, rocksdb::Error>
where
    P: AsRef<Path>,
    I: IntoIterator<Item = (N, Options)>,
    N: AsRef<str>,
{
    MultiThreadedDb::open_cf_with_opts(&new_db_options(), data_dir, cfs)
}

// TODO: rocksdb tuning? see:
// https://github.com/sei-protocol/sei-db/blob/main/ss/rocksdb/opts.go#L29-L65
// https://github.com/turbofish-org/merk/blob/develop/src/merk/mod.rs#L84-L102
fn new_db_options() -> Options {
    let mut opts = Options::default();
    opts.create_if_missing(true);
    opts.create_missing_column_families(true);
    opts
}
