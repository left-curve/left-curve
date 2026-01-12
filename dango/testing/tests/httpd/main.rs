use {
    actix_web::{
        App,
        body::MessageBody,
        dev::{ServiceFactory, ServiceRequest, ServiceResponse},
        web,
    },
    dango_httpd::{graphql::build_schema, server::config_app},
};

pub mod accounts;
pub mod candles;
pub mod grug;
pub mod metrics;
pub mod shutdown;
pub mod trades;
pub mod transfers;
pub mod users;

fn build_actix_app(
    dango_httpd_context: dango_httpd::context::Context,
) -> App<
    impl ServiceFactory<
        ServiceRequest,
        Response = ServiceResponse<impl MessageBody>,
        Config = (),
        InitError = (),
        Error = actix_web::Error,
    >,
> {
    let graphql_schema = build_schema(dango_httpd_context.clone());

    App::new()
        .app_data(web::Data::new(dango_httpd_context.clone()))
        .app_data(web::Data::new(
            dango_httpd_context.indexer_httpd_context.clone(),
        ))
        .app_data(web::Data::new(graphql_schema.clone()))
        .configure(config_app(dango_httpd_context, graphql_schema))
}
