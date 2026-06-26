use {
    async_graphql::{
        EmptyMutation, ObjectType, Schema, SchemaBuilder, SubscriptionType, dataloader::DataLoader,
    },
    dango_indexer_historical_block_source::{BlockLoader, BlockSource},
    sea_orm::DatabaseConnection,
    std::sync::Arc,
};

/// Assemble the read-only schema's shape — no context data, so it can be
/// introspected in tests without a live database. The query root (and, when a
/// projection contributes one, the subscription root) are built and merged by
/// the composition root; this only adds the server-wide conventions: no
/// mutations, and initial depth / complexity caps.
///
/// The caps are a coarse abuse guard, deliberately generous — the read surface
/// is shallow (connection → edges → node → `tx` / `outcome` / `data`) and every
/// list is `LIMIT`-bounded, so legitimate queries and introspection sit well
/// under them while pathological deep / wide documents are rejected before they
/// reach the resolvers (and their per-row block hydration). Tune as the schema
/// grows; per-field complexity weights can refine the breadth cap later.
fn assemble<Q, S>(query: Q, subscription: S) -> SchemaBuilder<Q, EmptyMutation, S>
where
    Q: ObjectType + 'static,
    S: SubscriptionType + 'static,
{
    Schema::build(query, EmptyMutation, subscription)
        .limit_depth(20)
        .limit_complexity(1_000)
}

/// Build the read-only schema from the roots the composition root assembled,
/// injecting the shared read-side handles as typed context data: the Postgres
/// pool (table queries), the block source (raw-payload hydration), and a
/// [`BlockLoader`] `DataLoader` over it (so resolvers hydrating a unit's `Tx` /
/// `TxOutcome` batch their block reads by height instead of an N+1). Resolvers
/// fetch them with `ctx.data::<…>()`.
pub fn build_schema<Q, S>(
    query: Q,
    subscription: S,
    db: DatabaseConnection,
    source: Arc<dyn BlockSource>,
) -> Schema<Q, EmptyMutation, S>
where
    Q: ObjectType + 'static,
    S: SubscriptionType + 'static,
{
    let blocks = DataLoader::new(BlockLoader::new(Arc::clone(&source)), tokio::spawn);
    assemble(query, subscription)
        .data(db)
        .data(source)
        .data(blocks)
        .finish()
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        async_graphql::{EmptySubscription, MergedObject},
        dango_indexer_historical_block_source::BlockQuery,
        dango_indexer_historical_projection::ActivityQuery,
    };

    // A stand-in for the root the composition root assembles: the core block
    // query plus the registered projections' query objects.
    #[derive(MergedObject, Default)]
    struct Query(BlockQuery, ActivityQuery);

    /// The read surface surfaces in the merged schema. Assembles without context
    /// data (introspection never runs resolvers) and checks the core `block`
    /// query and the activity feeds are present under their camelCased names — a
    /// guard for the merge as more queries / projections are added.
    #[test]
    fn schema_exposes_read_surface() {
        let schema = assemble(Query::default(), EmptySubscription).finish();
        let sdl = schema.sdl();

        // The core block-by-height query (not a projection): a `BlockData`
        // through the `JSON` scalar.
        assert!(
            sdl.contains("block(height: Int!): JSON"),
            "core `block` query missing from the merged root:\n{sdl}",
        );

        for field in [
            "transactionsByHash",
            "transactionsInvolving",
            "eventsByType",
            "contractEvents",
            "eventsInvolving",
            "contractEventsInvolving",
        ] {
            assert!(
                sdl.contains(field),
                "activity feed `{field}` missing from the merged root:\n{sdl}",
            );
        }
    }
}
