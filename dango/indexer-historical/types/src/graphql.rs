use {
    crate::AnyResult,
    anyhow::bail,
    dango_indexer_graphql_types::Variables,
    graphql_client::{GraphQLQuery, Response},
    serde::Serialize,
    std::fmt::Debug,
    url::Url,
};

/// POST a GraphQL query to `endpoint` and return the deserialised response data.
///
/// Generic over the query: pass any [`Variables`] impl (typically generated
/// by the `graphql_client` derive macro for a query in `dango-indexer-graphql-types`).
///
/// `endpoint` is expected to point directly at the GraphQL endpoint of the
/// target server (e.g. `http://localhost:8080/graphql`); the caller is in
/// charge of any path joining.
pub async fn post_graphql<V>(
    client: &reqwest::Client,
    endpoint: &Url,
    variables: V,
) -> AnyResult<<V::Query as GraphQLQuery>::ResponseData>
where
    V: Variables + Serialize + Debug,
    <V::Query as GraphQLQuery>::ResponseData: Debug,
{
    let query = V::Query::build_query(variables);

    let response = client.post(endpoint.clone()).json(&query).send().await?;

    if let Err(e) = response.error_for_status_ref() {
        bail!("{}: {}", e, response.text().await?);
    }

    let body: Response<<V::Query as GraphQLQuery>::ResponseData> = response.json().await?;

    match body.data {
        Some(data) => Ok(data),
        None => bail!("no data returned from query: errors: {:?}", body.errors),
    }
}
