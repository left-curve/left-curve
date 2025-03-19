use actix_web::{Responder, get};

#[get("/")]
pub async fn index() -> impl Responder {
    "OK"
}
