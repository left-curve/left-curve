use {
    async_graphql::Schema,
    dango_httpd::graphql::{query::Query, subscription::Subscription},
    indexer_httpd::graphql::mutation::Mutation,
};

fn main() {
    let schema = Schema::build(
        Query::default(),
        Mutation::default(),
        Subscription::default(),
    )
    .finish();

    let filename = std::env::args()
        .next_back()
        .expect("No argument given. Please provide the path to the schema file.");

    let sdl = schema.sdl();
    std::fs::write(&filename, sdl).unwrap();

    println!("Schema generated successfully at: {filename:?}");
}
