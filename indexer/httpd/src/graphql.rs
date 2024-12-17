use async_graphql::{extensions, EmptyMutation, EmptySubscription, Schema};
use query::Query;

pub mod query;

pub(crate) type AppSchema = Schema<Query, EmptyMutation, EmptySubscription>;

pub fn build_schema() -> AppSchema {
    Schema::build(Query::default(), EmptyMutation, EmptySubscription)
        .extension(extensions::Logger)
        .finish()
}
