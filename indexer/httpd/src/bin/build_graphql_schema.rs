use {
    async_graphql::Schema,
    indexer_httpd::graphql::{
        mutation::IndexerMutation, query::FullQuery, subscription::FullSubscription,
    },
};

fn main() {
    let schema = Schema::build(
        FullQuery::default(),
        IndexerMutation::default(),
        FullSubscription::default(),
    )
    .finish();

    let filename = std::env::args()
        .next_back()
        .expect("No argument given. Please provide the path to the schema file.");

    let sdl = schema.sdl();
    std::fs::write(&filename, sdl).unwrap();

    println!("Schema generated successfully at: {filename:?}");
}
