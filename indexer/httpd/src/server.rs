use {
    super::error::Error,
    crate::{context::Context, routes},
    actix_web::{App, HttpServer},
};

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

    HttpServer::new(move || {
        App::new()
            .service(routes::index::index)
            .app_data(context.clone())
    })
    .bind((ip, port))?
    .run()
    .await?;

    Ok(())
}
