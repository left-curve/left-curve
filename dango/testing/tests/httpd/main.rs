pub mod accounts;
pub mod transfers;

use {
    actix_web::{
        App,
        body::MessageBody,
        dev::{ServiceFactory, ServiceRequest, ServiceResponse},
    },
    dango_httpd::{graphql::build_schema, server::config_app},
    indexer_httpd::context::Context,
    indexer_testing::build_actix_app_with_config,
};

fn build_actix_app(
    app_ctx: Context,
) -> App<
    impl ServiceFactory<
        ServiceRequest,
        Response = ServiceResponse<impl MessageBody>,
        Config = (),
        InitError = (),
        Error = actix_web::Error,
    >,
> {
    let graphql_schema = build_schema(app_ctx.clone());

    build_actix_app_with_config(app_ctx, graphql_schema, |app_ctx, graphql_schema| {
        config_app(app_ctx, graphql_schema)
    })
}
