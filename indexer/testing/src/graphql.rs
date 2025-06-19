use {
    crate::{GraphQLCustomRequest, PaginatedResponse, build_app_service, call_graphql},
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
    let mut models = vec![];
    let mut after: Option<String> = None;
    let mut before: Option<String> = None;

    loop {
        let app = build_app_service(httpd_context.clone());

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

        let response = call_graphql::<PaginatedResponse<R>, _, _, _>(app, request_body).await?;

        match (first, last) {
            (Some(_), None) => {
                for edge in response.data.edges {
                    models.push(edge.node);
                }

                if !response.data.page_info.has_next_page {
                    break;
                }
                // If we are paginating with `first`, we use the end cursor for the next request
                after = Some(response.data.page_info.end_cursor);
            },
            (None, Some(_)) => {
                for edge in response.data.edges.into_iter().rev() {
                    models.push(edge.node);
                }

                if !response.data.page_info.has_previous_page {
                    break;
                }
                // If we are paginating with `last`, we use the start cursor for the next request
                before = Some(response.data.page_info.start_cursor);
            },
            _ => {},
        }
    }

    Ok(models)
}
