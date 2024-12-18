use actix_web::{get, Responder};

#[get("/")]
pub async fn index() -> impl Responder {
    "OK"
}
