use {
    async_graphql::Schema,
    indexer_httpd::graphql::{mutation, query, subscription},
};

fn main() {
    let schema = Schema::build(
        query::Query::default(),
        mutation::Mutation::default(),
        subscription::Subscription::default(),
    )
    .finish();

    let filename = std::env::args()
        .last()
        .expect("No argument given. Please provide the path to the schema file.");

    let sdl = schema.sdl();
    std::fs::write(&filename, sdl).unwrap();

    println!("Schema generated successfully at: {:?}", filename);
}
