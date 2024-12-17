use {
    super::error::Error,
    crate::{context::Context, graphql::build_schema, routes},
    actix_web::{middleware::Logger, App, HttpServer},
};

/// Run the HTTP server, includes GraphQL and REST endpoints.
pub async fn run_server(ip: Option<&str>, port: Option<u16>) -> Result<(), Error> {
    let port = port
        .or_else(|| {
            std::env::var("PORT")
                .ok()
                .and_then(|val| val.parse::<u16>().ok())
        })
        .unwrap_or(8080);
    let ip = ip.unwrap_or("0.0.0.0");

    let context = Context::new().await?;
    let graphql_schema = build_schema();

    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .service(routes::index::index)
            .service(routes::graphql::graphql_route())
            .app_data(context.clone())
            .app_data(graphql_schema.clone())
    })
    .bind((ip, port))?
    .run()
    .await?;

    Ok(())
}
