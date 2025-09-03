use actix_web::Resource;

// Use the generic GraphQL route from grug_httpd to avoid code duplication
pub fn generic_graphql_route() -> Resource {
    grug_httpd::routes::graphql::generic_graphql_route::<
        crate::graphql::query::Query,
        crate::graphql::mutation::Mutation,
        crate::graphql::subscription::Subscription,
    >()
}
