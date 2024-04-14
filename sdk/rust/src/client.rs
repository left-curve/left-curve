use {
    crate::{AdminOption, SigningKey},
    anyhow::{bail, ensure},
    cw_account::{QueryMsg, StateResponse},
    cw_merkle::Proof,
    cw_std::{
        from_json_slice, from_json_value, hash, to_json_value, to_json_vec, AccountResponse, Addr,
        Binary, Coin, Coins, Config, Hash, InfoResponse, Message, QueryRequest, QueryResponse, Tx,
        WasmRawResponse,
    },
    serde::{de::DeserializeOwned, ser::Serialize},
    std::any::type_name,
    tendermint::block::Height,
    tendermint_rpc::{
        endpoint::{abci_query::AbciQuery, block, block_results, broadcast::tx_sync, status, tx},
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

    async fn query(
        &self,
        path:   &str,
        data:   Vec<u8>,
        height: Option<u64>,
        prove:  bool,
    ) -> anyhow::Result<AbciQuery> {
        let height = height.map(|h| h.try_into()).transpose()?;
        let res = self.inner.abci_query(Some(path.into()), data, height, prove).await?;
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

    pub async fn query_store(
        &self,
        key:    Vec<u8>,
        height: Option<u64>,
        prove:  bool,
    ) -> anyhow::Result<(Option<Vec<u8>>, Option<Proof>)> {
        let res = self.query("/store", key.clone(), height, prove).await?;
        let value = if res.value.is_empty() {
            None
        } else {
            Some(res.value)
        };
        let proof = if prove {
            ensure!(res.proof.is_some());
            let proof = res.proof.unwrap();
            ensure!(proof.ops.len() == 1);
            ensure!(proof.ops[0].field_type == type_name::<Proof>());
            ensure!(proof.ops[0].key == key);
            Some(from_json_slice(&proof.ops[0].data)?)
        } else {
            None
        };
        Ok((value, proof))
    }

    pub async fn query_app(
        &self,
        req: &QueryRequest,
        height: Option<u64>,
    ) -> anyhow::Result<QueryResponse> {
        let res = self.query("/app", to_json_vec(req)?.to_vec(), height, false).await?;
        Ok(from_json_slice(res.value)?)
    }

    pub async fn query_info(&self, height: Option<u64>) -> anyhow::Result<InfoResponse> {
        let res = self.query_app(&QueryRequest::Info {}, height).await?;
        Ok(res.as_info())
    }

    pub async fn query_balance(
        &self,
        address: Addr,
        denom: String,
        height: Option<u64>,
    ) -> anyhow::Result<Coin> {
        let res = self.query_app(&QueryRequest::Balance { address, denom }, height).await?;
        Ok(res.as_balance())
    }

    pub async fn query_balances(
        &self,
        address: Addr,
        start_after: Option<String>,
        limit: Option<u32>,
        height: Option<u64>,
    ) -> anyhow::Result<Coins> {
        let res = self.query_app(&QueryRequest::Balances { address, start_after, limit }, height).await?;
        Ok(res.as_balances())
    }

    pub async fn query_supply(&self, denom: String, height: Option<u64>) -> anyhow::Result<Coin> {
        let res = self.query_app(&QueryRequest::Supply { denom }, height).await?;
        Ok(res.as_supply())
    }

    pub async fn query_supplies(
        &self,
        start_after: Option<String>,
        limit: Option<u32>,
        height: Option<u64>,
    ) -> anyhow::Result<Coins> {
        let res = self.query_app(&QueryRequest::Supplies { start_after, limit }, height).await?;
        Ok(res.as_supplies())
    }

    pub async fn query_code(&self, hash: Hash, height: Option<u64>) -> anyhow::Result<Binary> {
        let res = self.query_app(&QueryRequest::Code { hash }, height).await?;
        Ok(res.as_code())
    }

    pub async fn query_codes(
        &self,
        start_after: Option<Hash>,
        limit: Option<u32>,
        height: Option<u64>,
    ) -> anyhow::Result<Vec<Hash>> {
        let res = self.query_app(&QueryRequest::Codes { start_after, limit }, height).await?;
        Ok(res.as_codes())
    }

    pub async fn query_account(&self, address: Addr, height: Option<u64>) -> anyhow::Result<AccountResponse> {
        let res = self.query_app(&QueryRequest::Account { address }, height).await?;
        Ok(res.as_account())
    }

    pub async fn query_accounts(
        &self,
        start_after: Option<Addr>,
        limit: Option<u32>,
        height: Option<u64>,
    ) -> anyhow::Result<Vec<AccountResponse>> {
        let res = self.query_app(&QueryRequest::Accounts { start_after, limit }, height).await?;
        Ok(res.as_accounts())
    }

    pub async fn query_wasm_raw(
        &self,
        contract: Addr,
        key: Binary,
        height: Option<u64>,
    ) -> anyhow::Result<WasmRawResponse> {
        let res = self.query_app(&QueryRequest::WasmRaw { contract, key }, height).await?;
        Ok(res.as_wasm_raw())
    }

    pub async fn query_wasm_smart<M: Serialize, R: DeserializeOwned>(
        &self,
        contract: Addr,
        msg: &M,
        height: Option<u64>,
    ) -> anyhow::Result<R> {
        let msg = to_json_value(msg)?;
        let res = self.query_app(&QueryRequest::WasmSmart { contract, msg }, height).await?;
        Ok(from_json_value(res.as_wasm_smart().data)?)
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
            None => self.query_info(None).await?.chain_id,
            Some(id) => id.to_string(),
        };

        let sequence = match sign_opts.sequence {
            None => {
                self.query_wasm_smart::<_, StateResponse>(
                    sign_opts.sender.clone(),
                    &QueryMsg::State {},
                    None,
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
            let tx_bytes = to_json_vec(&tx)?;
            Ok(Some(self.inner.broadcast_tx_sync(tx_bytes).await?))
        } else {
            Ok(None)
        }
    }

    pub async fn set_config(
        &self,
        new_cfg: Config,
        sign_opts: &SigningOptions,
    ) -> anyhow::Result<tx_sync::Response> {
        self.send_tx(vec![Message::SetConfig { new_cfg }], sign_opts).await
    }

    pub async fn transfer(
        &self,
        to: Addr,
        coins: Coins,
        sign_opts: &SigningOptions
    ) -> anyhow::Result<tx_sync::Response> {
        self.send_tx(vec![Message::Transfer { to, coins }], sign_opts).await
    }

    pub async fn upload(
        &self,
        wasm_byte_code: Binary,
        sign_opts: &SigningOptions,
    ) -> anyhow::Result<tx_sync::Response> {
        self.send_tx(vec![Message::Upload { wasm_byte_code }], sign_opts).await
    }

    pub async fn instantiate<M: Serialize>(
        &self,
        code_hash: Hash,
        msg: &M,
        salt: Binary,
        funds: Coins,
        admin: AdminOption,
        sign_opts: &SigningOptions,
    ) -> anyhow::Result<(Addr, tx_sync::Response)> {
        let address = Addr::compute(&sign_opts.sender, &code_hash, &salt);
        let msg = to_json_value(msg)?;
        let admin = admin.decide(&Addr::compute(&sign_opts.sender, &code_hash, &salt));
        let res = self.send_tx(vec![Message::Instantiate { code_hash, msg, salt, funds, admin }], sign_opts).await?;
        Ok((address, res))
    }

    pub async fn upload_and_instantiate<M: Serialize>(
        &self,
        wasm_byte_code: Binary,
        msg: &M,
        salt: Binary,
        funds: Coins,
        admin: AdminOption,
        sign_opts: &SigningOptions,
    ) -> anyhow::Result<(Addr, tx_sync::Response)> {
        let code_hash = hash(&wasm_byte_code);
        let address = Addr::compute(&sign_opts.sender, &code_hash, &salt);
        let msg = to_json_value(msg)?;
        let admin = admin.decide(&address);
        let upload_msg = Message::Upload { wasm_byte_code };
        let instantiate_msg = Message::Instantiate { code_hash, msg, salt, funds, admin };
        let res = self.send_tx(vec![upload_msg, instantiate_msg], sign_opts).await?;
        Ok((address, res))
    }

    pub async fn execute<M: Serialize>(
        &self,
        contract: Addr,
        msg: &M,
        funds: Coins,
        sign_opts: &SigningOptions,
    ) -> anyhow::Result<tx_sync::Response> {
        let msg = to_json_value(msg)?;
        self.send_tx(vec![Message::Execute { contract, msg, funds }], sign_opts).await
    }

    pub async fn migrate<M: Serialize>(
        &self,
        contract: Addr,
        new_code_hash: Hash,
        msg: &M,
        sign_opts: &SigningOptions,
    ) -> anyhow::Result<tx_sync::Response> {
        let msg = to_json_value(msg)?;
        self.send_tx(vec![Message::Migrate { contract, new_code_hash, msg }], sign_opts).await
    }

    pub async fn create_client<A: Serialize, B: Serialize>(
        &self,
        code_hash: Hash,
        client_state: &A,
        consensus_state: &B,
        salt: Binary,
        sign_opts: &SigningOptions,
    ) -> anyhow::Result<(Addr, tx_sync::Response)> {
        let address = Addr::compute(&sign_opts.sender, &code_hash, &salt);
        let client_state = to_json_value(client_state)?;
        let consensus_state = to_json_value(consensus_state)?;
        let msg = Message::CreateClient {
            code_hash,
            client_state,
            consensus_state,
            salt,
        };
        let res = self.send_tx(vec![msg], sign_opts).await?;
        Ok((address, res))
    }

    pub async fn update_client<M: Serialize>(
        &self,
        client_id: Addr,
        header: &M,
        sign_opts: &SigningOptions,
    ) -> anyhow::Result<tx_sync::Response> {
        let msg = Message::UpdateClient {
            client_id,
            header: to_json_value(header)?,
        };
        self.send_tx(vec![msg], sign_opts).await
    }

    pub async fn freeze_client<M: Serialize>(
        &self,
        client_id: Addr,
        misbehavior: &M,
        sign_opts: &SigningOptions,
    ) -> anyhow::Result<tx_sync::Response> {
        let msg = Message::FreezeClient {
            client_id,
            misbehavior: to_json_value(misbehavior)?,
        };
        self.send_tx(vec![msg], sign_opts).await
    }
}
