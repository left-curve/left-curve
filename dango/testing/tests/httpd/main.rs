pub mod accounts;
pub mod grug;
pub mod transfers;
pub mod users;

use {
    actix_web::{
        App,
        body::MessageBody,
        dev::{ServiceFactory, ServiceRequest, ServiceResponse},
    },
    dango_httpd::{graphql::build_schema, server::config_app},
    indexer_httpd::context::Context,
    indexer_testing::{build_actix_app_with_config, paginate_models_with_app_builder},
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

async fn paginate_models<R>(
    httpd_context: Context,
    graphql_query: &str,
    name: &str,
    sort_by: &str,
    first: Option<i32>,
    last: Option<i32>,
) -> anyhow::Result<Vec<R>>
where
    R: serde::de::DeserializeOwned,
{
    paginate_models_with_app_builder(
        httpd_context,
        graphql_query,
        name,
        sort_by,
        first,
        last,
        build_actix_app,
    )
    .await
}
