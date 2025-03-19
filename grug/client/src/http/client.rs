use {
    super::query::{get_block, query_app},
    anyhow::anyhow,
    async_trait::async_trait,
    graphql_client::{GraphQLQuery, Response},
    grug_types::{
        Block, BlockClient, BlockInfo, BlockResult, Duration, Hash256, HexBinary, JsonDeExt,
        JsonSerExt, Proof, Query, QueryClient, QueryResponse,
    },
    serde::Serialize,
    std::str::FromStr,
};

use super::query::Variables;

pub struct HttpClient {
    inner: reqwest::Client,
    endpoint: String,
}

impl HttpClient {
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
impl QueryClient for HttpClient {
    type Error = anyhow::Error;

    async fn query_chain(
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

    async fn query_store(
        &self,
        key: HexBinary,
        height: Option<u64>,
        prove: bool,
    ) -> Result<(Option<Vec<u8>>, Option<Proof>), Self::Error> {
        todo!()
    }
}

#[async_trait]
impl BlockClient for HttpClient {
    type Error = anyhow::Error;

    async fn query_block(&self, height: Option<u64>) -> Result<Block, Self::Error> {
        let response = self
            .perform(get_block::Variables {
                height: height.unwrap_or_default() as i64,
            })
            .await?
            .block
            .ok_or(anyhow!("No block returned from query"))?;

        Ok(Block {
            info: BlockInfo {
                height: response.block_height as u64,
                timestamp: Duration::from_nanos(
                    response.created_at.timestamp_nanos_opt().unwrap() as u128
                ),
                hash: Hash256::from_str(&response.hash)?,
            },
            // TODO
            txs: vec![],
        })
    }

    async fn query_block_result(&self, height: Option<u64>) -> Result<BlockResult, Self::Error> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        grug_types::{QueryClient, QueryClientExt},
    };

    const GRAPHQL_URL: &str = "https://devnet-graphql.dango.exchange";

    #[tokio::test]
    async fn test_query_app() {
        let client = HttpClient::new(GRAPHQL_URL);

        let response = client.query_chain(Query::config(), None).await.unwrap();
        println!("{:?}", response);

        let response = client.query_block(Some(1)).await.unwrap();
        println!("{:?}", response);

        let response = client.query_config(None).await.unwrap();
        println!("{:?}", response);
    }
}
