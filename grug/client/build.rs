use {
    async_graphql::Schema,
    indexer_httpd::graphql::{mutation, query, subscription},
};

const SCHEMA_PATH: &str = "src/http/schemas/schema.graphql";

fn main() {
    let schema = Schema::build(
        query::Query::default(),
        mutation::Mutation::default(),
        subscription::Subscription::default(),
    )
    .finish();
    let sdl = schema.sdl();
    std::fs::write(SCHEMA_PATH, sdl).unwrap();

    println!(
        "Schema generated successfully at: {:?}",
        std::env::current_dir().unwrap().join(SCHEMA_PATH)
    );
}
