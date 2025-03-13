use {
    super::queries::{get_block, query_app, Variables},
    anyhow::anyhow,
    graphql_client::{GraphQLQuery, Response},
    grug_provider::Provider,
    grug_types::{BlockInfo, Duration, Hash256, JsonDeExt, JsonSerExt, Query, QueryResponse},
    sea_orm::prelude::async_trait::async_trait,
    serde::Serialize,
    std::str::FromStr,
};

pub struct Client {
    inner: reqwest::Client,
    endpoint: String,
}

impl Client {
    pub fn new(endpoint: &str) -> Self {
        Self {
            inner: reqwest::Client::new(),
            endpoint: endpoint.to_string(),
        }
    }

    async fn perform<V>(
        &self,
        variables: V,
    ) -> Result<<V::Query as GraphQLQuery>::ResponseData, anyhow::Error>
    where
        V: Variables + Serialize,
    {
        let query = V::Query::build_query(variables);
        let response = self.inner.post(&self.endpoint).json(&query).send().await?;

        let body: Response<<V::Query as GraphQLQuery>::ResponseData> = response.json().await?;

        match body.data {
            Some(data) => Ok(data),
            None => Err(anyhow::anyhow!(
                "No data returned from query: errors: {:?}",
                body.errors
            )),
        }
    }
}

#[async_trait]
impl Provider for Client {
    type Error = anyhow::Error;

    async fn query_app(
        &self,
        query: Query,
        height: Option<u64>,
    ) -> Result<QueryResponse, Self::Error> {
        let response = self
            .perform(query_app::Variables {
                request: query.to_json_string()?,
                height: height.unwrap_or_default() as i64,
            })
            .await?;

        Ok(response.query_app.deserialize_json()?)
    }

    async fn query_block(&self, height: Option<u64>) -> Result<BlockInfo, Self::Error> {
        let response = self
            .perform(get_block::Variables {
                height: height.unwrap_or_default() as i64,
            })
            .await?
            .block
            .ok_or(anyhow!("No block returned from query"))?;

        Ok(BlockInfo {
            height: response.block_height as u64,
            timestamp: Duration::from_nanos(
                response.created_at.timestamp_nanos_opt().unwrap() as u128
            ),
            hash: Hash256::from_str(&response.hash)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GRAPHQL_URL: &str = "https://devnet-graphql.dango.exchange";

    #[tokio::test]
    async fn test_query_app() {
        let client = Client::new(GRAPHQL_URL);

        let response = client.query_app(Query::config(), None).await.unwrap();
        println!("{:?}", response);

        let response = client.query_block(Some(1)).await.unwrap();
        println!("{:?}", response);
    }
}
