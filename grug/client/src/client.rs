use {
    crate::{AdminOption, GasOption, SigningOption},
    anyhow::{bail, ensure},
    grug_jmt::Proof,
    grug_math::Inner,
    grug_types::{
        Addr, AsyncSigner, Binary, Coin, Coins, Config, ConfigUpdates, ContractInfo, Denom,
        GenericResult, Hash256, HashExt, Json, JsonDeExt, JsonSerExt, Message, Op, Query,
        QueryResponse, StdError, Tx, TxOutcome, UnsignedTx,
    },
    serde::{de::DeserializeOwned, ser::Serialize},
    std::{any::type_name, collections::BTreeMap},
    tendermint::{block::Height, Hash as TmHash},
    tendermint_rpc::{
        endpoint::{abci_query::AbciQuery, block, block_results, broadcast::tx_sync, status, tx},
        Client as TmClient, HttpClient,
    },
};

/// A client for interacting with a Grug chain via Tendermint RPC.
///
/// Internally, this is a wrapper over [`tendermint_rpc::HttpClient`](tendermint_rpc::HttpClient).
pub struct Client {
    inner: HttpClient,
}

impl Client {
    /// Creating a new [`Client`](crate::Client) by connecting to a Tendermint
    /// RPC endpoint.
    pub fn connect(endpoint: &str) -> anyhow::Result<Self> {
        let inner = HttpClient::new(endpoint)?;
        Ok(Self { inner })
    }

    // -------------------------- tendermint methods ---------------------------

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

    // ----------------------------- query methods -----------------------------

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
        self.query_app(&Query::Config {}, height)
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
        self.query_app(&Query::Balance { address, denom }, height)
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
        self.query_app(
            &Query::Balances {
                address,
                start_after,
                limit,
            },
            height,
        )
        .await
        .map(|res| res.as_balances())
    }

    /// Query a token's total supply.
    pub async fn query_supply(&self, denom: Denom, height: Option<u64>) -> anyhow::Result<Coin> {
        self.query_app(&Query::Supply { denom }, height)
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
        self.query_app(&Query::Supplies { start_after, limit }, height)
            .await
            .map(|res| res.as_supplies())
    }

    /// Query a single Wasm byte code by hash.
    pub async fn query_code(&self, hash: Hash256, height: Option<u64>) -> anyhow::Result<Binary> {
        self.query_app(&Query::Code { hash }, height)
            .await
            .map(|res| res.as_code())
    }

    /// Enumerate hashes of all codes.
    pub async fn query_codes(
        &self,
        start_after: Option<Hash256>,
        limit: Option<u32>,
        height: Option<u64>,
    ) -> anyhow::Result<BTreeMap<Hash256, Binary>> {
        self.query_app(&Query::Codes { start_after, limit }, height)
            .await
            .map(|res| res.as_codes())
    }

    /// Query the metadata of a single contract.
    pub async fn query_contract(
        &self,
        address: Addr,
        height: Option<u64>,
    ) -> anyhow::Result<ContractInfo> {
        self.query_app(&Query::Contract { address }, height)
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
        self.query_app(&Query::Contracts { start_after, limit }, height)
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
        self.query_app(
            &Query::WasmRaw {
                contract,
                key: key.into(),
            },
            height,
        )
        .await
        .map(|res| res.as_wasm_raw())
    }

    /// Call the contract's query entry point with the given message.
    pub async fn query_wasm_smart<M, R>(
        &self,
        contract: Addr,
        msg: &M,
        height: Option<u64>,
    ) -> anyhow::Result<R>
    where
        M: Serialize,
        R: DeserializeOwned,
    {
        self.query_app(
            &Query::WasmSmart {
                contract,
                msg: msg.to_json_value()?,
            },
            height,
        )
        .await?
        .as_wasm_smart()
        .deserialize_json()
        .map_err(Into::into)
    }

    /// Simulate the gas usage of a transaction.
    pub async fn simulate(&self, unsigned_tx: &UnsignedTx) -> anyhow::Result<TxOutcome> {
        self.query("/simulate", unsigned_tx.to_json_vec()?, None, false)
            .await?
            .value
            .deserialize_json()
            .map_err(Into::into)
    }

    // -------------------------- transaction methods --------------------------

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

    /// Create, sign, and broadcast a transaction with a single message, without
    /// terminal prompt for confirmation.
    ///
    /// If you need the prompt confirmation, use `send_message_with_confirmation`.
    pub async fn send_message<S>(
        &self,
        msg: Message,
        gas_opt: GasOption,
        sign_opt: SigningOption<'_, S>,
    ) -> anyhow::Result<tx_sync::Response>
    where
        S: AsyncSigner,
    {
        self.send_messages(vec![msg], gas_opt, sign_opt).await
    }

    /// Create, sign, and broadcast a transaction with a single message, with
    /// terminal prompt for confirmation.
    ///
    /// Returns `None` if the prompt is denied.
    pub async fn send_message_with_confirmation<S>(
        &self,
        msg: Message,
        gas_opt: GasOption,
        sign_opt: SigningOption<'_, S>,
        confirm_fn: fn(&Tx) -> anyhow::Result<bool>,
    ) -> anyhow::Result<Option<tx_sync::Response>>
    where
        S: AsyncSigner,
    {
        self.send_messages_with_confirmation(vec![msg], gas_opt, sign_opt, confirm_fn)
            .await
    }

    /// Create, sign, and broadcast a transaction with the given messages,
    /// without terminal prompt for confirmation.
    ///
    /// If you need the prompt confirmation, use `send_messages_with_confirmation`.
    pub async fn send_messages<S>(
        &self,
        msgs: Vec<Message>,
        gas_opt: GasOption,
        sign_opt: SigningOption<'_, S>,
    ) -> anyhow::Result<tx_sync::Response>
    where
        S: AsyncSigner,
    {
        self.send_messages_with_confirmation(msgs, gas_opt, sign_opt, no_confirmation)
            .await
            .map(Option::unwrap)
    }

    /// Create, sign, and broadcast a transaction with the given messages, with
    /// terminal prompt for confirmation.
    ///
    /// Returns `None` if the prompt is denied.
    pub async fn send_messages_with_confirmation<S>(
        &self,
        msgs: Vec<Message>,
        gas_opt: GasOption,
        sign_opt: SigningOption<'_, S>,
        confirm_fn: fn(&Tx) -> anyhow::Result<bool>,
    ) -> anyhow::Result<Option<tx_sync::Response>>
    where
        S: AsyncSigner,
    {
        // If gas limit is not provided, simulate
        let gas_limit = match gas_opt {
            GasOption::Simulate {
                flat_increase,
                scale,
            } => {
                let unsigned_tx = UnsignedTx {
                    sender: sign_opt.sender,
                    msgs: msgs.clone(),
                    // TODO: allow user to specify this
                    data: Json::Null,
                };
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

        let tx = sign_opt
            .signing_key
            .sign_transaction(msgs, &sign_opt.chain_id, gas_limit)
            .await?;

        self.broadcast_tx_with_confirmation(tx, confirm_fn).await
    }

    /// Send a transaction with a single [`Message::Configure`](grug_types::Message::Configure).
    pub async fn configure<S>(
        &self,
        updates: ConfigUpdates,
        app_updates: BTreeMap<String, Op<Json>>,
        gas_opt: GasOption,
        sign_opt: SigningOption<'_, S>,
    ) -> anyhow::Result<tx_sync::Response>
    where
        S: AsyncSigner,
    {
        let msg = Message::configure(updates, app_updates);
        self.send_message(msg, gas_opt, sign_opt).await
    }

    /// Send a transaction with a single [`Message::Transfer`](grug_types::Message::Transfer).
    pub async fn transfer<C, S>(
        &self,
        to: Addr,
        coins: C,
        gas_opt: GasOption,
        sign_opt: SigningOption<'_, S>,
    ) -> anyhow::Result<tx_sync::Response>
    where
        C: TryInto<Coins>,
        S: AsyncSigner,
        StdError: From<C::Error>,
    {
        let msg = Message::transfer(to, coins)?;
        self.send_message(msg, gas_opt, sign_opt).await
    }

    /// Send a transaction with a single [`Message::Upload`](grug_types::Message::Upload).
    pub async fn upload<B, S>(
        &self,
        code: B,
        gas_opt: GasOption,
        sign_opt: SigningOption<'_, S>,
    ) -> anyhow::Result<tx_sync::Response>
    where
        B: Into<Binary>,
        S: AsyncSigner,
    {
        let msg = Message::upload(code);
        self.send_message(msg, gas_opt, sign_opt).await
    }

    /// Send a transaction with a single [`Message::Instantiate`](grug_types::Message::Instantiate).
    ///
    /// Return the deployed contract's address.
    pub async fn instantiate<M, S, C, T>(
        &self,
        code_hash: Hash256,
        msg: &M,
        salt: S,
        label: Option<&str>,
        funds: C,
        gas_opt: GasOption,
        sign_opt: SigningOption<'_, T>,
        admin_opt: AdminOption,
    ) -> anyhow::Result<(Addr, tx_sync::Response)>
    where
        M: Serialize,
        S: Into<Binary>,
        C: TryInto<Coins>,
        T: AsyncSigner,
        StdError: From<C::Error>,
    {
        let salt = salt.into();
        let address = Addr::derive(sign_opt.sender, code_hash, &salt);
        let admin = admin_opt.decide(address);

        let msg = Message::instantiate(code_hash, msg, salt, label, admin, funds)?;
        let res = self.send_message(msg, gas_opt, sign_opt).await?;

        Ok((address, res))
    }

    /// Send a transaction that uploads a Wasm code, then instantiate a contract
    /// with the code in one go.
    ///
    /// Return the code hash, and the deployed contract's address.
    pub async fn upload_and_instantiate<M, B, S, C, T>(
        &self,
        code: B,
        msg: &M,
        salt: S,
        label: Option<&str>,
        funds: C,
        gas_opt: GasOption,
        sign_opt: SigningOption<'_, T>,
        admin_opt: AdminOption,
    ) -> anyhow::Result<(Hash256, Addr, tx_sync::Response)>
    where
        M: Serialize,
        B: Into<Binary>,
        S: Into<Binary>,
        C: TryInto<Coins>,
        T: AsyncSigner,
        StdError: From<C::Error>,
    {
        let code = code.into();
        let code_hash = code.hash256();
        let salt = salt.into();
        let address = Addr::derive(sign_opt.sender, code_hash, &salt);
        let admin = admin_opt.decide(address);

        let msgs = vec![
            Message::upload(code),
            Message::instantiate(code_hash, msg, salt, label, admin, funds)?,
        ];
        let res = self.send_messages(msgs, gas_opt, sign_opt).await?;

        Ok((code_hash, address, res))
    }

    /// Send a transaction with a single [`Message::Execute`](grug_types::Message::Execute).
    pub async fn execute<M, C, S>(
        &self,
        contract: Addr,
        msg: &M,
        funds: C,
        gas_opt: GasOption,
        sign_opt: SigningOption<'_, S>,
    ) -> anyhow::Result<tx_sync::Response>
    where
        M: Serialize,
        C: TryInto<Coins>,
        S: AsyncSigner,
        StdError: From<C::Error>,
    {
        let msg = Message::execute(contract, msg, funds)?;
        self.send_message(msg, gas_opt, sign_opt).await
    }

    /// Send a transaction with a single [`Message::Migrate`](grug_types::Message::Migrate).
    pub async fn migrate<M, S>(
        &self,
        contract: Addr,
        new_code_hash: Hash256,
        msg: &M,
        gas_opt: GasOption,
        sign_opt: SigningOption<'_, S>,
    ) -> anyhow::Result<tx_sync::Response>
    where
        M: Serialize,
        S: AsyncSigner,
    {
        let msg = Message::migrate(contract, new_code_hash, msg)?;
        self.send_message(msg, gas_opt, sign_opt).await
    }
}

/// Skip the CLI prompt confirmation, always consider it as if the user accepted.
fn no_confirmation(_tx: &Tx) -> anyhow::Result<bool> {
    Ok(true)
}
