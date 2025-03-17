use {
    anyhow::bail,
    async_trait::async_trait,
    grug_types::{
        Base64Encoder, Block, BlockClient, BlockInfo, BroadcastClient, BroadcastTxOutcome,
        CheckTxOutcome, Encoder, Hash256, JsonDeExt, JsonSerExt, Query, QueryClient, QueryResponse,
        Timestamp, Tx,
    },
    tendermint::{abci::Code, block::Height},
    tendermint_rpc::{endpoint::abci_query::AbciQuery, Client, HttpClient},
};

pub struct RpcaClient {
    inner: HttpClient,
}

impl RpcaClient {
    pub fn new(endpoint: &str) -> anyhow::Result<Self> {
        let inner = HttpClient::new(endpoint)?;
        Ok(Self { inner })
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
impl QueryClient for RpcaClient {
    type Error = anyhow::Error;

    async fn query_chain(
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
}

#[async_trait]
impl BlockClient for RpcaClient {
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
}

#[async_trait]
impl BroadcastClient for RpcaClient {
    type Error = anyhow::Error;

    async fn broadcast_tx(&self, tx: Tx) -> Result<BroadcastTxOutcome, Self::Error> {
        let tx = tx.to_json_vec()?;

        let response = self.inner.broadcast_tx_sync(tx).await?;

        Ok(BroadcastTxOutcome {
            tx_hash: Hash256::from_inner(response.hash.as_bytes().try_into()?),
            check_tx: CheckTxOutcome {
                gas_limit: 0,
                gas_used: 0,
                result: if response.code == Code::Ok {
                    Ok(())
                } else {
                    Err(response.log)
                },
                events: Base64Encoder::ENCODING
                    .decode(&response.data)
                    .unwrap()
                    .deserialize_json()?,
            },
        })
    }
}
