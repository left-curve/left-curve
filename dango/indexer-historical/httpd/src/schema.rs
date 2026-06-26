use {
    async_graphql::{EmptyMutation, ObjectType, Schema, SchemaBuilder, SubscriptionType},
    dango_indexer_historical_block_source::BlockSource,
    sea_orm::DatabaseConnection,
    std::sync::Arc,
};

/// Assemble the read-only schema's shape — no context data, so it can be
/// introspected in tests without a live database. The query root (and, when a
/// projection contributes one, the subscription root) are built and merged by
/// the composition root; this only adds the server-wide conventions (no
/// mutations; complexity / depth caps land here later).
fn assemble<Q, S>(query: Q, subscription: S) -> SchemaBuilder<Q, EmptyMutation, S>
where
    Q: ObjectType + 'static,
    S: SubscriptionType + 'static,
{
    Schema::build(query, EmptyMutation, subscription)
}

/// Build the read-only schema from the roots the composition root assembled,
/// injecting the shared read-side handles as typed context data: the Postgres
/// pool (table queries) and the block source (raw-payload hydration).
/// Resolvers fetch them with `ctx.data::<…>()`.
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
    assemble(query, subscription).data(db).data(source).finish()
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        async_graphql::{EmptySubscription, MergedObject},
        dango_indexer_historical_projection::ActivityQuery,
    };

    // A stand-in for the root the composition root assembles from the
    // registered projections' query objects.
    #[derive(MergedObject, Default)]
    struct Query(ActivityQuery);

    /// A projection's query object surfaces in the merged schema. Assembles
    /// without context data (introspection never runs resolvers) and checks the
    /// activity feeds are present under their camelCased names — a guard for the
    /// merge as more projections are added.
    #[test]
    fn schema_exposes_projection_surface() {
        let schema = assemble(Query::default(), EmptySubscription).finish();
        let sdl = schema.sdl();

        for field in [
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
