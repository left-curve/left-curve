use {
    crate::{AdminOption, GasOption},
    anyhow::{bail, ensure},
    grug_jmt::Proof,
    grug_math::Inner,
    grug_types::{
        Addr, Binary, Code, Coin, Coins, Config, ContractInfo, Denom, GenericResult, Hash256,
        HashExt, JsonDeExt, JsonSerExt, Message, NonEmpty, Query, QueryRequest, QueryResponse,
        Signer, StdError, Tx, TxOutcome, UnsignedTx,
    },
    serde::{de::DeserializeOwned, ser::Serialize},
    std::{any::type_name, collections::BTreeMap, ops::Deref},
    tendermint::{block::Height, Hash as TmHash},
    tendermint_rpc::{
        endpoint::{abci_query::AbciQuery, block, block_results, broadcast::tx_sync, status, tx},
        Client as TmClient, HttpClient, HttpClientUrl,
    },
};

/// A client for interacting with a Grug chain via Tendermint RPC.
///
/// Internally, this is a wrapper over [`tendermint_rpc::HttpClient`](tendermint_rpc::HttpClient).
#[derive(Debug, Clone)]
pub struct RpcClient {
    inner: HttpClient,
}

impl RpcClient {
    /// Creating a new [`QueryClient`](crate::QueryClient) by connecting to a Tendermint
    /// RPC endpoint.
    pub fn connect<U>(endpoint: U) -> anyhow::Result<Self>
    where
        U: TryInto<HttpClientUrl, Error = tendermint_rpc::Error>,
    {
        Ok(Self {
            inner: HttpClient::new(endpoint)?,
        })
    }

    /// Query the Tendermint node, sync, and validator status.
    pub async fn query_status(&self) -> anyhow::Result<status::Response> {
        Ok(self.inner.status().await?)
    }

    /// Query a single transaction and its execution result by hash.
    pub async fn query_tx(&self, hash: Hash256) -> anyhow::Result<tx::Response> {
        Ok(self
            .inner
            .tx(TmHash::Sha256(hash.into_inner()), false)
            .await?)
    }

    /// Query a block by height.
    ///
    /// If height is `None`, the latest block is fetched.
    ///
    /// Note that this doesn't include the block's execution results, such as
    /// events.
    pub async fn query_block(&self, height: Option<u64>) -> anyhow::Result<block::Response> {
        match height {
            Some(height) => Ok(self.inner.block(Height::try_from(height)?).await?),
            None => Ok(self.inner.latest_block().await?),
        }
    }

    /// Query a block, as well as its execution results, by hash.
    ///
    /// If height is `None`, the latest block is fetched.
    pub async fn query_block_result(
        &self,
        height: Option<u64>,
    ) -> anyhow::Result<block_results::Response> {
        match height {
            Some(height) => Ok(self.inner.block_results(Height::try_from(height)?).await?),
            None => Ok(self.inner.latest_block_results().await?),
        }
    }

    /// Query the Grug app through the ABCI `Query` method.
    ///
    /// Used internally. Use `query_store` or `query_app` instead.
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

    /// Make a raw query at the Grug app's storage.
    ///
    /// ## Parameters
    ///
    /// - `key`: The raw storage key.
    /// - `height`: The block height to perform the query. If unspecified, the
    ///   latest height is used. Errors if the node has already pruned the height.
    /// - `proof`: Whether to request a Merkle proof. If the key exists, an
    ///   memership proof is returned; otherwise, a non-membership proof is returned.
    pub async fn query_store(
        &self,
        key: Vec<u8>,
        height: Option<u64>,
        prove: bool,
    ) -> anyhow::Result<(Option<Vec<u8>>, Option<Proof>)> {
        let res = self.query("/store", key.clone(), height, prove).await?;

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
            Some(res.value)
        };

        // Do some basic sanity checks of the Merkle proof returned, and
        // deserialize it.
        // If the Grug app works properly, these should always succeed.
        let proof = if prove {
            ensure!(res.proof.is_some());
            let proof = res.proof.unwrap();
            ensure!(proof.ops.len() == 1);
            ensure!(proof.ops[0].field_type == type_name::<Proof>());
            ensure!(proof.ops[0].key == key);
            Some(proof.ops[0].data.deserialize_json()?)
        } else {
            ensure!(res.proof.is_none());
            None
        };

        Ok((value, proof))
    }

    /// Query the Grug app.
    ///
    /// Used internally. Use the `query_{info,balance,wasm_smart,...}` methods
    /// instead.
    pub async fn query_app(
        &self,
        req: &Query,
        height: Option<u64>,
    ) -> anyhow::Result<QueryResponse> {
        self.query("/app", req.to_json_vec()?.to_vec(), height, false)
            .await?
            .value
            .deserialize_json()
            .map_err(Into::into)
    }

    /// Query the chain-level configuration.
    pub async fn query_config(&self, height: Option<u64>) -> anyhow::Result<Config> {
        self.query_app(&Query::config(), height)
            .await
            .map(|res| res.as_config())
    }

    /// Query an account's balance in a single denom.
    pub async fn query_balance(
        &self,
        address: Addr,
        denom: Denom,
        height: Option<u64>,
    ) -> anyhow::Result<Coin> {
        self.query_app(&Query::balance(address, denom), height)
            .await
            .map(|res| res.as_balance())
    }

    /// Enumerate an account's balances in all denoms
    pub async fn query_balances(
        &self,
        address: Addr,
        start_after: Option<Denom>,
        limit: Option<u32>,
        height: Option<u64>,
    ) -> anyhow::Result<Coins> {
        self.query_app(&Query::balances(address, start_after, limit), height)
            .await
            .map(|res| res.as_balances())
    }

    /// Query a token's total supply.
    pub async fn query_supply(&self, denom: Denom, height: Option<u64>) -> anyhow::Result<Coin> {
        self.query_app(&Query::supply(denom), height)
            .await
            .map(|res| res.as_supply())
    }

    /// Enumerate all token's total supplies.
    pub async fn query_supplies(
        &self,
        start_after: Option<Denom>,
        limit: Option<u32>,
        height: Option<u64>,
    ) -> anyhow::Result<Coins> {
        self.query_app(&Query::supplies(start_after, limit), height)
            .await
            .map(|res| res.as_supplies())
    }

    /// Query a single Wasm byte code by hash.
    pub async fn query_code(&self, hash: Hash256, height: Option<u64>) -> anyhow::Result<Code> {
        self.query_app(&Query::code(hash), height)
            .await
            .map(|res| res.as_code())
    }

    /// Enumerate hashes of all codes.
    pub async fn query_codes(
        &self,
        start_after: Option<Hash256>,
        limit: Option<u32>,
        height: Option<u64>,
    ) -> anyhow::Result<BTreeMap<Hash256, Code>> {
        self.query_app(&Query::codes(start_after, limit), height)
            .await
            .map(|res| res.as_codes())
    }

    /// Query the metadata of a single contract.
    pub async fn query_contract(
        &self,
        address: Addr,
        height: Option<u64>,
    ) -> anyhow::Result<ContractInfo> {
        self.query_app(&Query::contract(address), height)
            .await
            .map(|res| res.as_contract())
    }

    /// Enumerate metadata of all contracts.
    pub async fn query_contracts(
        &self,
        start_after: Option<Addr>,
        limit: Option<u32>,
        height: Option<u64>,
    ) -> anyhow::Result<BTreeMap<Addr, ContractInfo>> {
        self.query_app(&Query::contracts(start_after, limit), height)
            .await
            .map(|res| res.as_contracts())
    }

    /// Query a raw key-value pair in a contract's internal state.
    pub async fn query_wasm_raw<B>(
        &self,
        contract: Addr,
        key: B,
        height: Option<u64>,
    ) -> anyhow::Result<Option<Binary>>
    where
        B: Into<Binary>,
    {
        self.query_app(&Query::wasm_raw(contract, key), height)
            .await
            .map(|res| res.as_wasm_raw())
    }

    pub async fn query_wasm_smart<R>(
        &self,
        contract: Addr,
        req: R,
        height: Option<u64>,
    ) -> anyhow::Result<R::Response>
    where
        R: QueryRequest,
        R::Message: Serialize,
        R::Response: DeserializeOwned,
    {
        let msg = R::Message::from(req);

        self.query_app(&Query::wasm_smart(contract, &msg)?, height)
            .await
            .and_then(|res| res.as_wasm_smart().deserialize_json().map_err(Into::into))
    }

    /// Simulate the gas usage of a transaction.
    pub async fn simulate(&self, unsigned_tx: &UnsignedTx) -> anyhow::Result<TxOutcome> {
        self.query("/simulate", unsigned_tx.to_json_vec()?, None, false)
            .await?
            .value
            .deserialize_json()
            .map_err(Into::into)
    }

    /// Broadcast an already signed transaction, without terminal prompt for
    /// confirmation.
    pub async fn broadcast_tx(&self, tx: Tx) -> anyhow::Result<tx_sync::Response> {
        self.broadcast_tx_with_confirmation(tx, no_confirmation)
            .await
            .map(Option::unwrap)
    }

    /// Broadcast an already signed transaction, with terminal prompt for
    /// confirmation.
    pub async fn broadcast_tx_with_confirmation(
        &self,
        tx: Tx,
        confirm_fn: fn(&Tx) -> anyhow::Result<bool>,
    ) -> anyhow::Result<Option<tx_sync::Response>> {
        if confirm_fn(&tx)? {
            let tx_bytes = tx.to_json_vec()?;
            Ok(Some(self.inner.broadcast_tx_sync(tx_bytes).await?))
        } else {
            Ok(None)
        }
    }
}

/// A client for interacting with a Grug chain via Tendermint RPC, with the
/// additional capability of signing transactions.
#[derive(Debug, Clone)]
pub struct RpcSigningClient {
    inner: RpcClient,
    pub chain_id: String,
}

impl Deref for RpcSigningClient {
    type Target = RpcClient;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl RpcSigningClient {
    /// Creating a new [`Client`](crate::Client) by connecting to a Tendermint
    /// RPC endpoint.
    pub fn connect<T, U>(chain_id: T, endpoint: U) -> anyhow::Result<Self>
    where
        T: Into<String>,
        U: TryInto<HttpClientUrl, Error = tendermint_rpc::Error>,
    {
        Ok(Self {
            inner: RpcClient::connect(endpoint)?,
            chain_id: chain_id.into(),
        })
    }

    /// Create, sign, and broadcast a transaction with a single message, without
    /// terminal prompt for confirmation.
    ///
    /// If you need the prompt confirmation, use `send_message_with_confirmation`.
    pub async fn send_message<S>(
        &self,
        signer: &mut S,
        msg: Message,
        gas_opt: GasOption,
    ) -> anyhow::Result<tx_sync::Response>
    where
        S: Signer,
    {
        self.send_messages(signer, NonEmpty::new_unchecked(vec![msg]), gas_opt)
            .await
    }

    /// Create, sign, and broadcast a transaction with a single message, with
    /// terminal prompt for confirmation.
    ///
    /// Returns `None` if the prompt is denied.
    pub async fn send_message_with_confirmation<S>(
        &self,
        signer: &mut S,
        msg: Message,
        gas_opt: GasOption,
        confirm_fn: fn(&Tx) -> anyhow::Result<bool>,
    ) -> anyhow::Result<Option<tx_sync::Response>>
    where
        S: Signer,
    {
        self.send_messages_with_confirmation(
            signer,
            NonEmpty::new_unchecked(vec![msg]),
            gas_opt,
            confirm_fn,
        )
        .await
    }

    /// Create, sign, and broadcast a transaction with the given messages,
    /// without terminal prompt for confirmation.
    ///
    /// If you need the prompt confirmation, use `send_messages_with_confirmation`.
    pub async fn send_messages<S>(
        &self,
        signer: &mut S,
        msgs: NonEmpty<Vec<Message>>,
        gas_opt: GasOption,
    ) -> anyhow::Result<tx_sync::Response>
    where
        S: Signer,
    {
        self.send_messages_with_confirmation(signer, msgs, gas_opt, no_confirmation)
            .await
            .map(Option::unwrap)
    }

    /// Create, sign, and broadcast a transaction with the given messages, with
    /// terminal prompt for confirmation.
    ///
    /// Returns `None` if the prompt is denied.
    pub async fn send_messages_with_confirmation<S>(
        &self,
        signer: &mut S,
        msgs: NonEmpty<Vec<Message>>,
        gas_opt: GasOption,
        confirm_fn: fn(&Tx) -> anyhow::Result<bool>,
    ) -> anyhow::Result<Option<tx_sync::Response>>
    where
        S: Signer,
    {
        // If gas limit is not provided, simulate
        let gas_limit = match gas_opt {
            GasOption::Simulate {
                flat_increase,
                scale,
            } => {
                let unsigned_tx = signer.unsigned_transaction(msgs.clone(), &self.chain_id)?;
                match self.simulate(&unsigned_tx).await? {
                    TxOutcome {
                        result: GenericResult::Ok(_),
                        gas_used,
                        ..
                    } => (gas_used as f64 * scale).ceil() as u64 + flat_increase,
                    TxOutcome {
                        result: GenericResult::Err(err),
                        ..
                    } => bail!("Failed to estimate gas consumption: {err}"),
                }
            },
            GasOption::Predefined { gas_limit } => gas_limit,
        };

        let tx = signer.sign_transaction(msgs, &self.chain_id, gas_limit)?;

        self.broadcast_tx_with_confirmation(tx, confirm_fn).await
    }

    /// Send a transaction with a single [`Message::Configure`](grug_types::Message::Configure).
    pub async fn configure<S, T>(
        &self,
        signer: &mut S,
        new_cfg: Option<Config>,
        new_app_cfg: Option<T>,
        gas_opt: GasOption,
    ) -> anyhow::Result<tx_sync::Response>
    where
        S: Signer,
        T: Serialize,
    {
        let msg = Message::configure(new_cfg, new_app_cfg)?;
        self.send_message(signer, msg, gas_opt).await
    }

    /// Send a transaction with a single [`Message::Transfer`](grug_types::Message::Transfer).
    pub async fn transfer<S, C>(
        &self,
        signer: &mut S,
        to: Addr,
        coins: C,
        gas_opt: GasOption,
    ) -> anyhow::Result<tx_sync::Response>
    where
        S: Signer,
        C: TryInto<Coins>,
        StdError: From<C::Error>,
    {
        let msg = Message::transfer(to, coins)?;
        self.send_message(signer, msg, gas_opt).await
    }

    /// Send a transaction with a single [`Message::Upload`](grug_types::Message::Upload).
    pub async fn upload<S, B>(
        &self,
        signer: &mut S,
        code: B,
        gas_opt: GasOption,
    ) -> anyhow::Result<tx_sync::Response>
    where
        S: Signer,
        B: Into<Binary>,
    {
        let msg = Message::upload(code);
        self.send_message(signer, msg, gas_opt).await
    }

    /// Send a transaction with a single [`Message::Instantiate`](grug_types::Message::Instantiate).
    ///
    /// Return the deployed contract's address.
    pub async fn instantiate<S, M, SA, C>(
        &self,
        signer: &mut S,
        code_hash: Hash256,
        msg: &M,
        salt: SA,
        label: Option<&str>,
        funds: C,
        gas_opt: GasOption,
        admin_opt: AdminOption,
    ) -> anyhow::Result<(Addr, tx_sync::Response)>
    where
        S: Signer,
        M: Serialize,
        SA: Into<Binary>,
        C: TryInto<Coins>,

        StdError: From<C::Error>,
    {
        let salt = salt.into();
        let address = Addr::derive(signer.address(), code_hash, &salt);
        let admin = admin_opt.decide(address);

        let msg = Message::instantiate(code_hash, msg, salt, label, admin, funds)?;
        let res = self.send_message(signer, msg, gas_opt).await?;

        Ok((address, res))
    }

    /// Send a transaction that uploads a Wasm code, then instantiate a contract
    /// with the code in one go.
    ///
    /// Return the code hash, and the deployed contract's address.
    pub async fn upload_and_instantiate<S, M, B, SA, C>(
        &self,
        signer: &mut S,
        code: B,
        msg: &M,
        salt: SA,
        label: Option<&str>,
        funds: C,
        gas_opt: GasOption,
        admin_opt: AdminOption,
    ) -> anyhow::Result<(Hash256, Addr, tx_sync::Response)>
    where
        S: Signer,
        M: Serialize,
        B: Into<Binary>,
        SA: Into<Binary>,
        C: TryInto<Coins>,
        StdError: From<C::Error>,
    {
        let code = code.into();
        let code_hash = code.hash256();
        let salt = salt.into();
        let address = Addr::derive(signer.address(), code_hash, &salt);
        let admin = admin_opt.decide(address);

        let msgs = NonEmpty::new_unchecked(vec![
            Message::upload(code),
            Message::instantiate(code_hash, msg, salt, label, admin, funds)?,
        ]);
        let res = self.send_messages(signer, msgs, gas_opt).await?;

        Ok((code_hash, address, res))
    }

    /// Send a transaction with a single [`Message::Execute`](grug_types::Message::Execute).
    pub async fn execute<S, M, C>(
        &self,
        signer: &mut S,
        contract: Addr,
        msg: &M,
        funds: C,
        gas_opt: GasOption,
    ) -> anyhow::Result<tx_sync::Response>
    where
        S: Signer,
        M: Serialize,
        C: TryInto<Coins>,
        StdError: From<C::Error>,
    {
        let msg = Message::execute(contract, msg, funds)?;
        self.send_message(signer, msg, gas_opt).await
    }

    /// Send a transaction with a single [`Message::Migrate`](grug_types::Message::Migrate).
    pub async fn migrate<S, M>(
        &self,
        signer: &mut S,
        contract: Addr,
        new_code_hash: Hash256,
        msg: &M,
        gas_opt: GasOption,
    ) -> anyhow::Result<tx_sync::Response>
    where
        S: Signer,
        M: Serialize,
    {
        let msg = Message::migrate(contract, new_code_hash, msg)?;
        self.send_message(signer, msg, gas_opt).await
    }
}

/// Skip the CLI prompt confirmation, always consider it as if the user accepted.
fn no_confirmation(_tx: &Tx) -> anyhow::Result<bool> {
    Ok(true)
}
