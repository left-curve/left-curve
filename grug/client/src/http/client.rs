use {
    super::query::{query_app, query_store, simulate},
    async_trait::async_trait,
    graphql_client::{GraphQLQuery, Response},
    grug_math::Inner,
    grug_types::{
        Binary, Block, BlockClient, BlockOutcome, BorshDeExt, JsonDeExt, JsonSerExt, Proof, Query,
        QueryAppClient, QueryResponse, TxOutcome, UnsignedTx,
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

    async fn get(&self, path: &str) -> Result<reqwest::Response, anyhow::Error> {
        Ok(self
            .inner
            .get(format!("{}/{}", self.endpoint, path))
            .send()
            .await?)
    }

    async fn post_graphql<V>(
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
impl QueryAppClient for HttpClient {
    type Error = anyhow::Error;

    async fn query_chain(
        &self,
        query: Query,
        height: Option<u64>,
    ) -> Result<QueryResponse, Self::Error> {
        let response = self
            .post_graphql(query_app::Variables {
                request: query.to_json_string()?,
                height: height.map(|h| h as i64),
            })
            .await?;

        Ok(response.query_app.deserialize_json()?)
    }

    async fn query_store(
        &self,
        key: Binary,
        height: Option<u64>,
        prove: bool,
    ) -> Result<(Option<Binary>, Option<Proof>), Self::Error> {
        let response = self
            .post_graphql(query_store::Variables {
                key: key.to_string(),
                height: height.map(|h| h as i64),
                prove,
            })
            .await?;

        let proof = match response.query_store.proof {
            Some(proof) => Binary::from_str(&proof)?.into_inner().deserialize_borsh()?,
            None => None,
        };

        Ok((Some(Binary::from_str(&response.query_store.value)?), proof))
    }

    async fn simulate(&self, tx: UnsignedTx) -> Result<TxOutcome, Self::Error> {
        let response = self
            .post_graphql(simulate::Variables {
                tx: tx.to_json_string()?,
            })
            .await?;

        Ok(response.simulate.deserialize_json()?)
    }
}

#[async_trait]
impl BlockClient for HttpClient {
    type Error = anyhow::Error;

    async fn query_block(&self, height: Option<u64>) -> Result<Block, Self::Error> {
        let path = match height {
            Some(height) => format!("api/block/info/{}", height),
            None => "api/block/info".to_string(),
        };

        Ok(self.get(&path).await?.json().await?)
    }

    async fn query_block_result(&self, height: Option<u64>) -> Result<BlockOutcome, Self::Error> {
        let path = match height {
            Some(height) => format!("api/block/result/{}", height),
            None => "api/block/result".to_string(),
        };

        Ok(self.get(&path).await?.json().await?)
    }
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        grug_types::{QueryAppClient, QueryClientExt},
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
