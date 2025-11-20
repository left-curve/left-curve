mod migrate_wasm_cf;

use {
    crate::{CF_NAME_MIGRATIONS, DbError},
    rocksdb::{ColumnFamily, DB},
};

pub(crate) const MIGRATION_DONE_KEY: &[u8] = b"1";

pub(crate) fn run_migrations(db: &DB) -> Result<(), DbError> {
    migrate_wasm_cf::run(db)
}

fn cf_migrations(db: &DB) -> &ColumnFamily {
    db.cf_handle(CF_NAME_MIGRATIONS).unwrap_or_else(|| {
        panic!("failed to find migrations column family");
    })
}
