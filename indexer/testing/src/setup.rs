use indexer_hooked::HookedIndexer;

pub async fn create_hooked_indexer() -> (HookedIndexer, indexer_sql::Context, indexer_cache::Context)
{
    let sql_indexer = indexer_sql::IndexerBuilder::default()
        .with_memory_database()
        .build()
        .await
        .expect("Can't create indexer");

    let sql_indexer_context = sql_indexer.context.clone();

    let cache_indexer = indexer_cache::Cache::new_with_tempdir();
    let indexer_cache_context = cache_indexer.context.clone();

    let mut hooked_indexer = HookedIndexer::new();
    hooked_indexer.add_indexer(cache_indexer).unwrap();
    hooked_indexer.add_indexer(sql_indexer).unwrap();

    (hooked_indexer, sql_indexer_context, indexer_cache_context)
}
