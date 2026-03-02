use {
    async_graphql::Schema,
    dango_httpd::graphql::{query::Query, subscription::Subscription},
    indexer_httpd::graphql::mutation::Mutation,
};

fn main() -> std::io::Result<()> {
    let schema = Schema::build(
        Query::default(),
        Mutation::default(),
        Subscription::default(),
    )
    .finish();

    let filename = std::env::args().next_back().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "missing output path argument for schema file",
        )
    })?;

    let sdl = schema.sdl();
    std::fs::write(&filename, sdl)?;

    println!("Schema generated successfully at: {filename:?}");
    Ok(())
}
