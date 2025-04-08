use {
    anyhow::{bail, ensure},
    async_trait::async_trait,
    grug_math::Inner,
    grug_types::{
        Binary, Block, BlockClient, BlockInfo, BlockOutcome, BroadcastClient, BroadcastTxOutcome,
        CronOutcome, Hash256, JsonDeExt, JsonSerExt, Query, QueryClient, QueryResponse,
        SearchTxClient, SearchTxOutcome, StdResult, Timestamp, Tx, TxOutcome, UnsignedTx,
    },
    std::any::type_name,
    tendermint::block::Height,
    tendermint_rpc::{
        Client, HttpClient,
        endpoint::{abci_query::AbciQuery, status},
    },
};

#[derive(Debug, Clone)]
pub struct TendermintRpcClient {
    inner: HttpClient,
}

impl TendermintRpcClient {
    pub fn new(endpoint: &str) -> anyhow::Result<Self> {
        let inner = HttpClient::new(endpoint)?;
        Ok(Self { inner })
    }

    pub async fn status(&self) -> anyhow::Result<status::Response> {
        Ok(self.inner.status().await?)
    }

    pub async fn query(
        &self,
        path: &str,
        data: Vec<u8>,
        height: Option<u64>,
        prove: bool,
    ) -> anyhow::Result<AbciQuery> {
        let height = height.map(|h| h.try_into()).transpose()?;
        let res = self
            .inner
            .abci_query(Some(path.into()), data, height, prove)
            .await?;

        if res.code.is_err() {
            bail!(
                "query failed! codespace = {}, code = {}, log = {}",
                res.codespace,
                res.code.value(),
                res.log
            );
        }

        Ok(res)
    }
}

#[async_trait]
impl QueryClient for TendermintRpcClient {
    type Error = anyhow::Error;
    type Proof = grug_jmt::Proof;

    async fn query_app(
        &self,
        query: Query,
        height: Option<u64>,
    ) -> Result<QueryResponse, Self::Error> {
        self.query("/app", query.to_json_vec()?.to_vec(), height, false)
            .await?
            .value
            .deserialize_json()
            .map_err(Into::into)
    }

    async fn query_store(
        &self,
        key: Binary,
        height: Option<u64>,
        prove: bool,
    ) -> Result<(Option<Binary>, Option<Self::Proof>), Self::Error> {
        let res = self
            .query("/store", key.clone().into_inner(), height, prove)
            .await?;

        // The ABCI query always return the value as a `Vec<u8>`.
        // If the key doesn't exist, the value would be an empty vector.
        //
        // NOTE: This means that the Grug app must make sure values can't be
        // empty, otherwise in this query we can't tell whether it's that the
        // key oesn't exist, or it exists but the value is empty.
        //
        // See discussion in CosmWasm:
        // <https://github.com/CosmWasm/cosmwasm/blob/v2.1.0/packages/std/src/imports.rs#L142-L144>
        //
        // And my rant here:
        // <https://x.com/larry0x/status/1813287621449183651>
        let value = if res.value.is_empty() {
            None
        } else {
            Some(Binary::from_inner(res.value))
        };

        // Do some basic sanity checks of the Merkle proof returned, and
        // deserialize it.
        // If the Grug app works properly, these should always succeed.
        let proof = if prove {
            ensure!(res.proof.is_some());
            let proof = res.proof.unwrap();
            ensure!(proof.ops.len() == 1);
            ensure!(proof.ops[0].field_type == type_name::<Self::Proof>());
            ensure!(proof.ops[0].key == key.into_inner());
            Some(proof.ops[0].data.deserialize_json()?)
        } else {
            ensure!(res.proof.is_none());
            None
        };

        Ok((value, proof))
    }

    async fn simulate(&self, tx: UnsignedTx) -> Result<TxOutcome, Self::Error> {
        Ok(self
            .query("/simulate", tx.to_json_vec()?, None, false)
            .await?
            .value
            .deserialize_json()?)
    }
}

#[async_trait]
impl BlockClient for TendermintRpcClient {
    type Error = anyhow::Error;

    async fn query_block(&self, height: Option<u64>) -> Result<Block, Self::Error> {
        let response = match height {
            Some(height) => self.inner.block(Height::try_from(height)?).await?,
            None => self.inner.latest_block().await?,
        };

        Ok(Block {
            info: BlockInfo {
                height: response.block.header.height.into(),
                timestamp: Timestamp::from_nanos(
                    response.block.header.time.unix_timestamp_nanos() as u128
                ),
                hash: Hash256::from_inner(response.block.header.app_hash.as_bytes().try_into()?),
            },
            txs: response
                .block
                .data
                .iter()
                .map(|tx| {
                    let tx: Tx = tx.deserialize_json()?;
                    let tx_hash = tx.tx_hash()?;
                    anyhow::Ok((tx, tx_hash))
                })
                .collect::<Result<Vec<(Tx, Hash256)>, _>>()?,
        })
    }

    async fn query_block_outcome(&self, height: Option<u64>) -> Result<BlockOutcome, Self::Error> {
        let response = match height {
            Some(height) => self.inner.block_results(Height::try_from(height)?).await?,
            None => self.inner.latest_block_results().await?,
        };

        Ok(BlockOutcome {
            app_hash: Hash256::from_inner(response.app_hash.as_bytes().try_into()?),
            tx_outcomes: response
                .txs_results
                .unwrap_or_default()
                .into_iter()
                .map(TxOutcome::from_tm_tx_result)
                .collect::<StdResult<Vec<TxOutcome>>>()?,
            cron_outcomes: response
                .finalize_block_events
                .into_iter()
                .map(CronOutcome::from_tm_event)
                .collect::<StdResult<Vec<CronOutcome>>>()?,
        })
    }
}

#[async_trait]
impl BroadcastClient for TendermintRpcClient {
    type Error = anyhow::Error;

    async fn broadcast_tx(&self, tx: Tx) -> Result<BroadcastTxOutcome, Self::Error> {
        let response = self.inner.broadcast_tx_sync(tx.to_json_vec()?).await?;
        let outcome = BroadcastTxOutcome::from_tm_broadcast_response(response)?;

        Ok(outcome)
    }
}

#[async_trait]
impl SearchTxClient for TendermintRpcClient {
    type Error = anyhow::Error;

    async fn search_tx(&self, hash: Hash256) -> Result<SearchTxOutcome, Self::Error> {
        let response = self
            .inner
            .tx(tendermint::Hash::Sha256(hash.into_inner()), false)
            .await?;
        let outcome = SearchTxOutcome::from_tm_query_tx_response(response)?;

        Ok(outcome)
    }
}
