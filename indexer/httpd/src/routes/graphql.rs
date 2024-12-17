use crate::graphql::query::index::{graphiql_playgound, graphql_index, graphql_ws};
use actix_web::{web, Resource};

pub fn graphql_route() -> Resource {
    web::resource("/graphql")
        .route(web::post().to(graphql_index))
        .route(
            web::get()
                .guard(actix_web::guard::Header("upgrade", "websocket"))
                .to(graphql_ws),
        )
        .route(web::get().to(graphiql_playgound))
}
