use {
    crate::{AdminOption, SigningKey},
    anyhow::bail,
    cw_account::{QueryMsg, StateResponse},
    cw_std::{
        from_json, hash, to_json, AccountResponse, Addr, Binary, Coin, Coins, Config, Hash, InfoResponse, Message, QueryRequest, QueryResponse, Tx, WasmRawResponse
    },
    serde::{de::DeserializeOwned, ser::Serialize},
    tendermint::block::Height,
    tendermint_rpc::{
        endpoint::{block, block_results, broadcast::tx_sync, status, tx},
        Client as ClientTrait, HttpClient,
    },
};

pub struct SigningOptions {
    pub signing_key: SigningKey,
    pub sender:      Addr,
    pub chain_id:    Option<String>,
    pub sequence:    Option<u32>,
}

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

    pub async fn query(&self, req: &QueryRequest) -> anyhow::Result<QueryResponse> {
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

    pub async fn query_info(&self) -> anyhow::Result<InfoResponse> {
        let res = self.query(&QueryRequest::Info {}).await?;
        Ok(res.as_info())
    }

    pub async fn query_balance(&self, address: Addr, denom: String) -> anyhow::Result<Coin> {
        let res = self.query(&QueryRequest::Balance { address, denom }).await?;
        Ok(res.as_balance())
    }

    pub async fn query_balances(
        &self,
        address: Addr,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> anyhow::Result<Coins> {
        let res = self.query(&QueryRequest::Balances { address, start_after, limit }).await?;
        Ok(res.as_balances())
    }

    pub async fn query_supply(&self, denom: String) -> anyhow::Result<Coin> {
        let res = self.query(&QueryRequest::Supply { denom }).await?;
        Ok(res.as_supply())
    }

    pub async fn query_supplies(
        &self,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> anyhow::Result<Coins> {
        let res = self.query(&QueryRequest::Supplies { start_after, limit }).await?;
        Ok(res.as_supplies())
    }

    pub async fn query_code(&self, hash: Hash) -> anyhow::Result<Binary> {
        let res = self.query(&QueryRequest::Code { hash }).await?;
        Ok(res.as_code())
    }

    pub async fn query_codes(
        &self,
        start_after: Option<Hash>,
        limit: Option<u32>,
    ) -> anyhow::Result<Vec<Hash>> {
        let res = self.query(&QueryRequest::Codes { start_after, limit }).await?;
        Ok(res.as_codes())
    }

    pub async fn query_account(&self, address: Addr) -> anyhow::Result<AccountResponse> {
        let res = self.query(&QueryRequest::Account { address }).await?;
        Ok(res.as_account())
    }

    pub async fn query_accounts(
        &self,
        start_after: Option<Addr>,
        limit: Option<u32>,
    ) -> anyhow::Result<Vec<AccountResponse>> {
        let res = self.query(&QueryRequest::Accounts { start_after, limit }).await?;
        Ok(res.as_accounts())
    }

    pub async fn query_wasm_raw(
        &self,
        contract: Addr,
        key: Binary,
    ) -> anyhow::Result<WasmRawResponse> {
        let res = self.query(&QueryRequest::WasmRaw { contract, key }).await?;
        Ok(res.as_wasm_raw())
    }

    pub async fn query_wasm_smart<M: Serialize, R: DeserializeOwned>(
        &self,
        contract: Addr,
        msg: &M,
    ) -> anyhow::Result<R> {
        let msg = to_json(msg)?;
        let res = self.query(&QueryRequest::WasmSmart { contract, msg }).await?;
        Ok(from_json(res.as_wasm_smart().data)?)
    }

    // ------------------------------ tx methods -------------------------------

    /// Create, sign, and broadcast a transaction without confirmation.
    ///
    /// If you need the user to provide a confirmation (e.g. via CLI) before
    /// broadcasting, use `send_tx_with_confirmation`.
    pub async fn send_tx(
        &self,
        msgs: Vec<Message>,
        sign_opts: &SigningOptions,
    ) -> anyhow::Result<tx_sync::Response> {
        let maybe_res = self.send_tx_with_confirmation(msgs, sign_opts, |_| Ok(true)).await?;
        Ok(maybe_res.unwrap())
    }

    pub async fn send_tx_with_confirmation(
        &self,
        msgs: Vec<Message>,
        sign_opts: &SigningOptions,
        confirm_fn: fn(&Tx) -> anyhow::Result<bool>,
    ) -> anyhow::Result<Option<tx_sync::Response>> {
        let chain_id = match &sign_opts.chain_id {
            None => self.query_info().await?.chain_id,
            Some(id) => id.to_string(),
        };

        let sequence = match sign_opts.sequence {
            None => {
                self.query_wasm_smart::<_, StateResponse>(
                    sign_opts.sender.clone(),
                    &QueryMsg::State {},
                )
                .await?
                .sequence
            },
            Some(seq) => seq,
        };

        let tx = sign_opts.signing_key.create_and_sign_tx(
            msgs,
            sign_opts.sender.clone(),
            &chain_id,
            sequence,
        )?;

        if confirm_fn(&tx)? {
            let tx_bytes = to_json(&tx)?;
            Ok(Some(self.inner.broadcast_tx_sync(tx_bytes).await?))
        } else {
            Ok(None)
        }
    }

    pub async fn update_config(
        &self,
        new_cfg: Config,
        sign_opts: &SigningOptions,
    ) -> anyhow::Result<tx_sync::Response> {
        self.send_tx(vec![Message::UpdateConfig { new_cfg }], sign_opts).await
    }

    pub async fn transfer(
        &self,
        to: Addr,
        coins: Coins,
        sign_opts: &SigningOptions
    ) -> anyhow::Result<tx_sync::Response> {
        self.send_tx(vec![Message::Transfer { to, coins }], sign_opts).await
    }

    pub async fn store_code(
        &self,
        wasm_byte_code: Binary,
        sign_opts: &SigningOptions,
    ) -> anyhow::Result<tx_sync::Response> {
        self.send_tx(vec![Message::StoreCode { wasm_byte_code }], sign_opts).await
    }

    pub async fn instantiate<M: Serialize>(
        &self,
        code_hash: Hash,
        msg: &M,
        salt: Binary,
        funds: Coins,
        admin: AdminOption,
        sign_opts: &SigningOptions,
    ) -> anyhow::Result<tx_sync::Response> {
        let msg = to_json(msg)?;
        let admin = admin.decide(&Addr::compute(&sign_opts.sender, &code_hash, &salt));
        self.send_tx(vec![Message::Instantiate { code_hash, msg, salt, funds, admin }], sign_opts).await
    }

    pub async fn store_code_and_instantiate<M: Serialize>(
        &self,
        wasm_byte_code: Binary,
        msg: &M,
        salt: Binary,
        funds: Coins,
        admin: AdminOption,
        sign_opts: &SigningOptions,
    ) -> anyhow::Result<(Addr, tx_sync::Response)> {
        let code_hash = hash(&wasm_byte_code);
        let msg = to_json(msg)?;
        let address = Addr::compute(&sign_opts.sender, &code_hash, &salt);
        let admin = admin.decide(&address);
        let store_code_msg = Message::StoreCode { wasm_byte_code };
        let instantiate_msg = Message::Instantiate { code_hash, msg, salt, funds, admin };
        let res = self.send_tx(vec![store_code_msg, instantiate_msg], sign_opts).await?;
        Ok((address, res))
    }

    pub async fn execute<M: Serialize>(
        &self,
        contract: Addr,
        msg: &M,
        funds: Coins,
        sign_opts: &SigningOptions,
    ) -> anyhow::Result<tx_sync::Response> {
        let msg = to_json(msg)?;
        self.send_tx(vec![Message::Execute { contract, msg, funds }], sign_opts).await
    }

    pub async fn migrate<M: Serialize>(
        &self,
        contract: Addr,
        new_code_hash: Hash,
        msg: &M,
        sign_opts: &SigningOptions,
    ) -> anyhow::Result<tx_sync::Response> {
        let msg = to_json(msg)?;
        self.send_tx(vec![Message::Migrate { contract, new_code_hash, msg }], sign_opts).await
    }
}
