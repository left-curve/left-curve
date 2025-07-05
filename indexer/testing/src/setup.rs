use {indexer_hooked::HookedIndexer, indexer_sql::indexer_path::IndexerPath};

pub fn create_hooked_indexer(
    keep_blocks: bool,
) -> (HookedIndexer, indexer_sql::Context, IndexerPath) {
    let indexer = indexer_sql::IndexerBuilder::default()
        .with_memory_database()
        .with_keep_blocks(keep_blocks)
        .build()
        .expect("Can't create indexer");

    let indexer_context = indexer.context.clone();
    let indexer_path = indexer.indexer_path.clone();

    let mut hooked_indexer = HookedIndexer::new();
    hooked_indexer.add_indexer(indexer).unwrap();

    (hooked_indexer, indexer_context, indexer_path)
}
