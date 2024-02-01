use {
    anyhow::bail,
    cw_std::{from_json, to_json, QueryRequest, QueryResponse},
    tendermint::block::Height,
    tendermint_rpc::{
        endpoint::{block, block_results, status, tx}, Client as ClientTrait, HttpClient,
    },
};

pub struct Client {
    inner: HttpClient,
}

impl Client {
    pub fn connect(endpoint: &str) -> anyhow::Result<Self> {
        let inner = HttpClient::new(endpoint)?;
        Ok(Self {
            inner,
        })
    }

    // -------------------------- tendermint methods ---------------------------

    pub async fn status(&self) -> anyhow::Result<status::Response> {
        Ok(self.inner.status().await?)
    }

    pub async fn tx(&self, hash_str: &str) -> anyhow::Result<tx::Response> {
        let hash_bytes = hex::decode(hash_str)?;
        Ok(self.inner.tx(hash_bytes.try_into()?, false).await?)
    }

    pub async fn block(&self, height: Option<u64>) -> anyhow::Result<block::Response> {
        match height {
            Some(height) => Ok(self.inner.block(Height::try_from(height)?).await?),
            None => Ok(self.inner.latest_block().await?),
        }
    }

    pub async fn block_result(
        &self,
        height: Option<u64>,
    ) -> anyhow::Result<block_results::Response> {
        match height {
            Some(height) => Ok(self.inner.block_results(Height::try_from(height)?).await?),
            None => Ok(self.inner.latest_block_results().await?),
        }
    }

    // ----------------------------- query methods -----------------------------

    pub async fn query(&self, req: QueryRequest) -> anyhow::Result<QueryResponse> {
        let res = self.inner.abci_query(Some("app".into()), to_json(&req)?, None, false).await?;
        if res.code.is_err() {
            bail!(
                "query failed! codespace = {}, code = {}, log = {}",
                res.codespace,
                res.code.value(),
                res.log
            );
        }
        Ok(from_json(&res.value)?)
    }
}
