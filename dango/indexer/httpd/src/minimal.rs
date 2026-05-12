use {
    crate::graphql::query::grug::GrugQuery,
    actix_web::{
        HttpResponse,
        web::{self, ServiceConfig},
    },
    async_graphql::{EmptyMutation, EmptySubscription, Schema},
    grug_httpd::context::Context,
};

pub type MinimalSchema = Schema<GrugQuery, EmptyMutation, EmptySubscription>;

pub fn build_schema(ctx: Context) -> MinimalSchema {
    Schema::build(GrugQuery::default(), EmptyMutation, EmptySubscription)
        .data(ctx)
        .finish()
}

pub fn config_app(ctx: Context, schema: MinimalSchema) -> Box<dyn Fn(&mut ServiceConfig)> {
    Box::new(move |cfg: &mut ServiceConfig| {
        cfg.service(grug_httpd::routes::index::index)
            .service(grug_httpd::routes::index::up)
            .service(grug_httpd::routes::index::requester_ip)
            .service(grug_httpd::routes::graphql::graphql_route::<
                GrugQuery,
                EmptyMutation,
                EmptySubscription,
            >())
            .default_service(web::to(HttpResponse::NotFound))
            .app_data(web::Data::new(ctx.clone()))
            .app_data(web::Data::new(schema.clone()));
    })
}
