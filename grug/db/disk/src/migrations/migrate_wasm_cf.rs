use {
    crate::{
        DbError, cf_state_storage, cf_wasm_storage, create_state_iter,
        migrations::{MIGRATION_DONE_KEY, cf_migrations},
    },
    grug_types::{Order, increment_last_byte},
    rocksdb::{DB, WriteBatch},
};

const MIGRATION_NAME: &[u8] = b"migrate_wasm_cf";

pub(crate) fn run(db: &DB) -> Result<(), DbError> {
    #[cfg(feature = "tracing")]
    tracing::info!("Running migration wasm cf");

    let migrations_cf = cf_migrations(db);

    if let Some(done) = db.get_cf(migrations_cf, MIGRATION_NAME)? {
        if done == MIGRATION_DONE_KEY {
            #[cfg(feature = "tracing")]
            tracing::info!("Migration wasm cf already done");
            return Ok(());
        }
    }

    let wasm_cf = cf_wasm_storage(db);
    let state_cf = cf_state_storage(db);

    let mut batch = WriteBatch::new();

    for (k, v) in create_state_iter(
        db,
        Some(b"wasm"),
        Some(&increment_last_byte(b"wasm".to_vec())),
        Order::Ascending,
    ) {
        batch.put_cf(&wasm_cf, k.clone(), v);
        batch.delete_cf(&state_cf, k);
    }

    batch.put_cf(migrations_cf, MIGRATION_NAME, MIGRATION_DONE_KEY);

    db.write(batch)?;

    #[cfg(feature = "tracing")]
    tracing::info!("Migration wasm cf done");

    Ok(())
}
