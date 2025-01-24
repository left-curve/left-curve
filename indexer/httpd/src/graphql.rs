use {
    crate::context::Context,
    async_graphql::{extensions, EmptyMutation, EmptySubscription, MergedObject, Schema},
    query::Query,
};

pub mod query;
pub mod types;

// #[derive(MergedObject, Default, Clone)]
// pub struct BlankQueryHook();

pub(crate) type AppSchema = Schema<Query, EmptyMutation, EmptySubscription>;

// pub trait QueryDyn: async_graphql::ObjectType + Default + 'static {}
// pub trait SubscriptionDyn: async_graphql::SubscriptionType + Default + 'static {}
// pub(crate) type AppSchemaDyn = Schema<dyn QueryDyn, dyn QueryDyn, dyn SubscriptionDyn>;

pub fn build_schema(app_ctx: Context) -> AppSchema {
    Schema::build(Query::default(), EmptyMutation, EmptySubscription)
        .extension(extensions::Logger)
        .data(app_ctx)
        .finish()
}

// pub(crate) type AppSchemaWithSub<S> = Schema<QueryWithSub<S>, EmptyMutation, EmptySubscription>;

// pub fn build_schema_with_sub<S>(
//     app_ctx: Context,
//     // sub: Box<dyn async_graphql::ObjectType>,
// ) -> Schema<QueryWithSub<S>, EmptyMutation, EmptySubscription>
// where
//     S: async_graphql::ObjectType + Default + 'static,
// {
//     // #[derive(MergedObject, Default)]
//     // struct Query(Query, S);

//     Schema::build(
//         QueryWithSub::<S>::default(),
//         EmptyMutation,
//         EmptySubscription,
//     )
//     .extension(extensions::Logger)
//     .data(app_ctx)
//     .finish()
// }
