use {
    crate::{Variables, broadcast_tx_sync, query_app, query_store, search_tx, simulate},
    anyhow::{anyhow, bail},
    async_trait::async_trait,
    graphql_client::{GraphQLQuery, Response},
    grug_types::{
        Addr, Binary, Block, BlockClient, BlockOutcome, BorshDeExt, BroadcastClient,
        BroadcastTxOutcome, GenericResult, Hash256, Inner, Json, JsonDeExt, JsonSerExt, NonEmpty,
        Query, QueryClient, QueryResponse, SearchTxClient, SearchTxOutcome, Tx, TxOutcome,
        UnsignedTx,
    },
    serde::Serialize,
    std::{fmt::Display, str::FromStr},
};

#[derive(Debug, Clone)]
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

    async fn get<P>(&self, path: P) -> Result<reqwest::Response, anyhow::Error>
    where
        P: Display,
    {
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
        V: Variables + Serialize + std::fmt::Debug,
        <<V as crate::types::Variables>::Query as graphql_client::GraphQLQuery>::ResponseData:
            std::fmt::Debug,
    {
        let query = V::Query::build_query(variables);
        let response = self
            .inner
            .post(format!("{}/graphql", self.endpoint))
            .json(&query)
            .send()
            .await?;

        #[cfg(feature = "tracing")]
        {
            tracing::debug!("GraphQL request: {query:#?}");
            tracing::debug!("GraphQL response: {response:#?}");
        }

        let body: Response<<V::Query as GraphQLQuery>::ResponseData> = response.json().await?;

        match body.data {
            Some(data) => {
                #[cfg(feature = "tracing")]
                tracing::debug!("GraphQL body response: {data:#?}");

                Ok(data)
            },
            None => bail!("no data returned from query: errors: {:?}", body.errors),
        }
    }
}

#[async_trait]
impl QueryClient for HttpClient {
    type Error = anyhow::Error;
    type Proof = grug_types::Proof;

    async fn query_app(
        &self,
        query: Query,
        height: Option<u64>,
    ) -> Result<QueryResponse, Self::Error> {
        let response = self
            .post_graphql(query_app::Variables {
                request: query.to_json_value()?.into_inner(),
                height: height.map(|h| h as i64),
            })
            .await?;

        // TODO
        Ok(serde_json::from_value(response.query_app)?)
    }

    async fn query_store(
        &self,
        key: Binary,
        height: Option<u64>,
        prove: bool,
    ) -> Result<(Option<Binary>, Option<Self::Proof>), Self::Error> {
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
                tx: tx.to_json_value()?.into_inner(),
            })
            .await?;

        Ok(serde_json::from_value(response.simulate)?)
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

    async fn query_block_outcome(&self, height: Option<u64>) -> Result<BlockOutcome, Self::Error> {
        let path = match height {
            Some(height) => format!("api/block/result/{}", height),
            None => "api/block/result".to_string(),
        };

        Ok(self.get(&path).await?.json().await?)
    }
}

#[async_trait]
impl BroadcastClient for HttpClient {
    type Error = anyhow::Error;

    async fn broadcast_tx(&self, tx: Tx) -> Result<BroadcastTxOutcome, Self::Error> {
        let response = self
            .post_graphql(broadcast_tx_sync::Variables {
                tx: tx.to_json_value()?.into_inner(),
            })
            .await?
            .broadcast_tx_sync;

        Ok(serde_json::from_value(response)?)
    }
}

#[async_trait]
impl SearchTxClient for HttpClient {
    type Error = anyhow::Error;

    async fn search_tx(&self, hash: Hash256) -> Result<SearchTxOutcome, Self::Error> {
        let response = self
            .post_graphql(search_tx::Variables {
                hash: hash.to_string(),
            })
            .await?
            .transactions
            .nodes;

        let res = response.first().ok_or(anyhow!("no tx found"))?;

        let msgs = res
            .messages
            .iter()
            .map(|m| Json::from_inner(m.data.clone()).deserialize_json())
            .collect::<Result<Vec<_>, _>>()?;

        let tx = Tx {
            sender: Addr::from_str(&res.sender)?,
            gas_limit: res.gas_wanted as u64,
            msgs: NonEmpty::new(msgs)?,
            data: Json::from_inner(res.data.clone()),
            credential: Json::from_inner(res.credential.clone()),
        };

        Ok(SearchTxOutcome {
            hash,
            height: res.block_height as u64,
            index: res.transaction_idx as u32,
            tx,
            outcome: TxOutcome {
                gas_limit: res.gas_wanted as u64,
                gas_used: res.gas_used as u64,
                result: if res.has_succeeded {
                    GenericResult::Ok(())
                } else {
                    GenericResult::Err(res.error_message.clone().unwrap_or_default())
                },
                events: res
                    .nested_events
                    .clone()
                    .ok_or(anyhow!("no nested events"))?
                    .deserialize_json()?,
            },
        })
    }
}
