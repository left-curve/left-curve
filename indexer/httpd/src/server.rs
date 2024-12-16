use {
    super::error::Error,
    actix_web::{get, App, HttpServer, Responder},
};

#[get("/")]
async fn index() -> impl Responder {
    "OK"
}

pub async fn run_server(ip: Option<&str>, port: Option<u16>) -> Result<(), Error> {
    let port = port
        .or_else(|| {
            std::env::var("PORT")
                .ok()
                .and_then(|val| val.parse::<u16>().ok())
        })
        .unwrap_or(8080);
    let ip = ip.unwrap_or("0.0.0.0");

    HttpServer::new(|| App::new().service(index))
        .bind((ip, port))?
        .run()
        .await?;

    Ok(())
}
