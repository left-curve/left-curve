use {
    crate::{GraphQLCustomRequest, PaginatedResponse, build_app_service, call_paginated_graphql},
    actix_web::{
        body::MessageBody,
        dev::{AppConfig, ServiceFactory, ServiceResponse},
    },
    indexer_httpd::context::Context,
    serde_json::json,
};

pub async fn paginate_models<R>(
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
        build_app_service,
    )
    .await
}

pub async fn paginate_models_with_app_builder<R, A, S, B, F>(
    httpd_context: Context,
    graphql_query: &str,
    name: &str,
    sort_by: &str,
    first: Option<i32>,
    last: Option<i32>,
    app_builder: F,
) -> anyhow::Result<Vec<R>>
where
    R: serde::de::DeserializeOwned,
    F: Fn(Context) -> A,
    A: actix_service::IntoServiceFactory<S, actix_http::Request>,
    S: ServiceFactory<
            actix_http::Request,
            Config = AppConfig,
            Response = ServiceResponse<B>,
            Error = actix_web::Error,
        >,
    S::InitError: std::fmt::Debug,
    B: MessageBody,
{
    let mut models = vec![];
    let mut after: Option<String> = None;
    let mut before: Option<String> = None;

    loop {
        let app = app_builder(httpd_context.clone());

        let variables = json!({
              "first": first,
              "last": last,
              "sortBy": sort_by,
              "after": after,
              "before": before,
        })
        .as_object()
        .unwrap()
        .clone();

        let request_body = GraphQLCustomRequest {
            name,
            query: graphql_query,
            variables,
        };

        let response: PaginatedResponse<R> = call_paginated_graphql(app, request_body).await?;

        match (first, last) {
            (Some(_), None) => {
                for edge in response.edges {
                    models.push(edge.node);
                }

                if !response.page_info.has_next_page {
                    break;
                }
                // If we are paginating with `first`, we use the end cursor for the next request
                after = Some(response.page_info.end_cursor);
            },
            (None, Some(_)) => {
                for edge in response.edges.into_iter().rev() {
                    models.push(edge.node);
                }

                if !response.page_info.has_previous_page {
                    break;
                }
                // If we are paginating with `last`, we use the start cursor for the next request
                before = Some(response.page_info.start_cursor);
            },
            _ => {},
        }
    }

    Ok(models)
}
