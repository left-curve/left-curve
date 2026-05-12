pub use indexer_httpd::graphql::build_full_schema as build_schema;

pub mod query {
    pub use indexer_httpd::graphql::query::FullQuery as Query;
}

pub mod subscription {
    pub use indexer_httpd::graphql::subscription::FullSubscription as Subscription;
}
