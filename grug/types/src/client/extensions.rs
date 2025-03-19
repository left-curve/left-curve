use {
    super::{
        AdminOption, BroadcastClient, BroadcastTxOutcome, GasOption, QueryClient, SimulateClient,
        WithChainId,
    },
    crate::{
        Addr, Binary, Code, Coins, Config, ContractInfo, Denom, GenericResult, Hash256, HashExt,
        JsonDeExt, Message, NonEmpty, Query, QueryRequest, QueryResponse, Signer, StdError, Tx,
        TxOutcome,
    },
    async_trait::async_trait,
    grug_math::Uint128,
    serde::{de::DeserializeOwned, Serialize},
    std::collections::BTreeMap,
};

// ----------------------------- Extension traits ------------------------------

#[async_trait]
pub trait QueryClientExt: QueryClient
where
    Self::Error: From<StdError>,
{
    async fn query_config(&self, height: Option<u64>) -> Result<Config, Self::Error> {
        self.query_chain(Query::config(), height)
            .await
            .map(|res| res.as_config())
    }

    async fn query_owner(&self, height: Option<u64>) -> Result<Addr, Self::Error> {
        self.query_config(height).await.map(|res| res.owner)
    }

    async fn query_bank(&self, height: Option<u64>) -> Result<Addr, Self::Error> {
        self.query_config(height).await.map(|res| res.bank)
    }

    async fn query_taxman(&self, height: Option<u64>) -> Result<Addr, Self::Error> {
        self.query_config(height).await.map(|res| res.taxman)
    }

    async fn query_app_config<T>(&self, height: Option<u64>) -> Result<T, Self::Error>
    where
        T: DeserializeOwned,
    {
        self.query_chain(Query::app_config(), height)
            .await
            .and_then(|res| res.as_app_config().deserialize_json().map_err(Into::into))
    }

    async fn query_balance(
        &self,
        address: Addr,
        denom: Denom,
        height: Option<u64>,
    ) -> Result<Uint128, Self::Error> {
        self.query_chain(Query::balance(address, denom), height)
            .await
            .map(|res| res.as_balance().amount)
    }

    async fn query_balances(
        &self,
        address: Addr,
        start_after: Option<Denom>,
        limit: Option<u32>,
        height: Option<u64>,
    ) -> Result<Coins, Self::Error> {
        self.query_chain(Query::balances(address, start_after, limit), height)
            .await
            .map(|res| res.as_balances())
    }

    async fn query_supply(
        &self,
        denom: Denom,
        height: Option<u64>,
    ) -> Result<Uint128, Self::Error> {
        self.query_chain(Query::supply(denom), height)
            .await
            .map(|res| res.as_supply().amount)
    }

    async fn query_supplies(
        &self,
        start_after: Option<Denom>,
        limit: Option<u32>,
        height: Option<u64>,
    ) -> Result<Coins, Self::Error> {
        self.query_chain(Query::supplies(start_after, limit), height)
            .await
            .map(|res| res.as_supplies())
    }

    async fn query_code(&self, hash: Hash256, height: Option<u64>) -> Result<Code, Self::Error> {
        self.query_chain(Query::code(hash), height)
            .await
            .map(|res| res.as_code())
    }

    async fn query_codes(
        &self,
        start_after: Option<Hash256>,
        limit: Option<u32>,
        height: Option<u64>,
    ) -> Result<BTreeMap<Hash256, Code>, Self::Error> {
        self.query_chain(Query::codes(start_after, limit), height)
            .await
            .map(|res| res.as_codes())
    }

    async fn query_contract(
        &self,
        address: Addr,
        height: Option<u64>,
    ) -> Result<ContractInfo, Self::Error> {
        self.query_chain(Query::contract(address), height)
            .await
            .map(|res| res.as_contract())
    }

    async fn query_contracts(
        &self,
        start_after: Option<Addr>,
        limit: Option<u32>,
        height: Option<u64>,
    ) -> Result<BTreeMap<Addr, ContractInfo>, Self::Error> {
        self.query_chain(Query::contracts(start_after, limit), height)
            .await
            .map(|res| res.as_contracts())
    }

    /// Note: In most cases, for querying a single storage path in another
    /// contract, the `StorageQuerier::query_wasm_path` method is preferred.
    ///
    /// The only case where `query_wasm_raw` is preferred is if you just want to
    /// know whether a data exists or not, without needing to deserialize it.
    async fn query_wasm_raw<B>(
        &self,
        contract: Addr,
        key: B,
        height: Option<u64>,
    ) -> Result<Option<Binary>, Self::Error>
    where
        B: Into<Binary> + Send,
    {
        self.query_chain(Query::wasm_raw(contract, key), height)
            .await
            .map(|res| res.as_wasm_raw())
    }

    async fn query_wasm_smart<R>(
        &self,
        contract: Addr,
        req: R,
        height: Option<u64>,
    ) -> Result<R::Response, Self::Error>
    where
        R: QueryRequest + Send,
        R::Message: Serialize + Send,
        R::Response: DeserializeOwned,
    {
        let msg = R::Message::from(req);

        self.query_chain(Query::wasm_smart(contract, &msg)?, height)
            .await
            .and_then(|res| res.as_wasm_smart().deserialize_json().map_err(Into::into))
    }

    async fn query_multi<const N: usize>(
        &self,
        requests: [Query; N],
        height: Option<u64>,
    ) -> Result<[Result<QueryResponse, Self::Error>; N], Self::Error> {
        self.query_chain(Query::Multi(requests.into()), height)
            .await
            .map(|res| {
                // We trust that the host has properly implemented the multi
                // query method, meaning the number of responses should always
                // match the number of requests.
                let res = res.as_multi();

                assert_eq!(
                    res.len(),
                    N,
                    "number of responses ({}) does not match that of requests ({})",
                    res.len(),
                    N
                );

                let mut iter = res.into_iter();

                std::array::from_fn(|_| {
                    iter.next()
                    .unwrap() // unwrap is safe because we've checked the length.
                    .map_err(StdError::host)
                    .map_err(Into::into)
                })
            })
    }
}

impl<C> QueryClientExt for C
where
    C: QueryClient,
    C::Error: From<StdError>,
{
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum GasEstimateError {
    #[error("Failed to estimate gas consumption: {0}")]
    GasEstimate(String),
}

#[async_trait]
pub trait BroadcastClientExt: BroadcastClient + SimulateClient + WithChainId
where
    <Self as BroadcastClient>::Error:
        From<GasEstimateError> + From<StdError> + From<<Self as SimulateClient>::Error>,
{
    async fn broadcast_tx_with_confirmation<F>(
        &self,
        tx: Tx,
        confirm_fn: F,
    ) -> Result<Option<BroadcastTxOutcome>, <Self as BroadcastClient>::Error>
    where
        F: Fn(&Tx) -> Result<bool, <Self as BroadcastClient>::Error> + Send + Sync,
    {
        if confirm_fn(&tx)? {
            self.broadcast_tx(tx).await.map(Some)
        } else {
            Ok(None)
        }
    }

    /// Create, sign, and broadcast a transaction with a single message, without
    /// terminal prompt for confirmation.
    ///
    /// If you need the prompt confirmation, use `send_message_with_confirmation`.
    async fn send_message<S>(
        &self,
        signer: &mut S,
        msg: Message,
        gas_opt: GasOption,
    ) -> Result<BroadcastTxOutcome, <Self as BroadcastClient>::Error>
    where
        S: Signer + Send + Sync,
    {
        self.send_messages(signer, NonEmpty::new_unchecked(vec![msg]), gas_opt)
            .await
    }

    /// Create, sign, and broadcast a transaction with a single message, with
    /// terminal prompt for confirmation.
    ///
    /// Returns `None` if the prompt is denied.
    async fn send_message_with_confirmation<S, F>(
        &self,
        signer: &mut S,
        msg: Message,
        gas_opt: GasOption,
        confirm_fn: F,
    ) -> Result<Option<BroadcastTxOutcome>, <Self as BroadcastClient>::Error>
    where
        S: Signer + Send + Sync,
        F: Fn(&Tx) -> Result<bool, <Self as BroadcastClient>::Error> + Send + Sync,
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
    async fn send_messages<S>(
        &self,
        signer: &mut S,
        msgs: NonEmpty<Vec<Message>>,
        gas_opt: GasOption,
    ) -> Result<BroadcastTxOutcome, <Self as BroadcastClient>::Error>
    where
        S: Signer + Send + Sync,
    {
        self.send_messages_with_confirmation(signer, msgs, gas_opt, no_confirmation)
            .await
            .map(Option::unwrap)
    }

    /// Create, sign, and broadcast a transaction with the given messages, with
    /// terminal prompt for confirmation.
    ///
    /// Returns `None` if the prompt is denied.
    async fn send_messages_with_confirmation<S, F>(
        &self,
        signer: &mut S,
        msgs: NonEmpty<Vec<Message>>,
        gas_opt: GasOption,
        confirm_fn: F,
    ) -> Result<Option<BroadcastTxOutcome>, <Self as BroadcastClient>::Error>
    where
        S: Signer + Send + Sync,
        F: Fn(&Tx) -> Result<bool, <Self as BroadcastClient>::Error> + Send + Sync,
    {
        // If gas limit is not provided, simulate
        let gas_limit = match gas_opt {
            GasOption::Simulate {
                flat_increase,
                scale,
            } => {
                let unsigned_tx = signer.unsigned_transaction(msgs.clone(), self.chain_id())?;
                match self.simulate(unsigned_tx).await? {
                    TxOutcome {
                        result: GenericResult::Ok(_),
                        gas_used,
                        ..
                    } => (gas_used as f64 * scale).ceil() as u64 + flat_increase,
                    TxOutcome {
                        result: GenericResult::Err(err),
                        ..
                    // } => return Err(format!("Failed to estimate gas consumption: {err}").into()),
                    } => return Err(GasEstimateError::GasEstimate(err).into()),
                }
            },
            GasOption::Predefined { gas_limit } => gas_limit,
        };

        let tx = signer.sign_transaction(msgs, &self.chain_id(), gas_limit)?;

        self.broadcast_tx_with_confirmation(tx, confirm_fn).await
    }

    /// Send a transaction with a single [`Message::Configure`](grug_types::Message::Configure).
    async fn configure<S, T>(
        &self,
        signer: &mut S,
        new_cfg: Option<Config>,
        new_app_cfg: Option<T>,
        gas_opt: GasOption,
    ) -> Result<BroadcastTxOutcome, <Self as BroadcastClient>::Error>
    where
        S: Signer + Send + Sync,
        T: Serialize + Send,
    {
        let msg = Message::configure(new_cfg, new_app_cfg)?;
        self.send_message(signer, msg, gas_opt).await
    }

    /// Send a transaction with a single [`Message::Transfer`](grug_types::Message::Transfer).
    async fn transfer<S, C>(
        &self,
        signer: &mut S,
        to: Addr,
        coins: C,
        gas_opt: GasOption,
    ) -> Result<BroadcastTxOutcome, <Self as BroadcastClient>::Error>
    where
        S: Signer + Send + Sync,
        C: TryInto<Coins> + Send,
        StdError: From<C::Error>,
    {
        let msg = Message::transfer(to, coins)?;
        self.send_message(signer, msg, gas_opt).await
    }

    /// Send a transaction with a single [`Message::Upload`](grug_types::Message::Upload).
    async fn upload<S, B>(
        &self,
        signer: &mut S,
        code: B,
        gas_opt: GasOption,
    ) -> Result<BroadcastTxOutcome, <Self as BroadcastClient>::Error>
    where
        S: Signer + Send + Sync,
        B: Into<Binary> + Send,
    {
        let msg = Message::upload(code);
        self.send_message(signer, msg, gas_opt).await
    }

    /// Send a transaction with a single [`Message::Instantiate`](grug_types::Message::Instantiate).
    ///
    /// Return the deployed contract's address.
    async fn instantiate<S, M, SA, C>(
        &self,
        signer: &mut S,
        code_hash: Hash256,
        msg: &M,
        salt: SA,
        label: Option<&str>,
        funds: C,
        gas_opt: GasOption,
        admin_opt: AdminOption,
    ) -> Result<(Addr, BroadcastTxOutcome), <Self as BroadcastClient>::Error>
    where
        S: Signer + Send + Sync,
        M: Serialize + Send + Sync,
        SA: Into<Binary> + Send,
        C: TryInto<Coins> + Send,

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
    async fn upload_and_instantiate<S, M, B, SA, C>(
        &self,
        signer: &mut S,
        code: B,
        msg: &M,
        salt: SA,
        label: Option<&str>,
        funds: C,
        gas_opt: GasOption,
        admin_opt: AdminOption,
    ) -> Result<(Hash256, Addr, BroadcastTxOutcome), <Self as BroadcastClient>::Error>
    where
        S: Signer + Send + Sync,
        M: Serialize + Send + Sync,
        B: Into<Binary> + Send,
        SA: Into<Binary> + Send,
        C: TryInto<Coins> + Send,
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
    async fn execute<S, M, C>(
        &self,
        signer: &mut S,
        contract: Addr,
        msg: &M,
        funds: C,
        gas_opt: GasOption,
    ) -> Result<BroadcastTxOutcome, <Self as BroadcastClient>::Error>
    where
        S: Signer + Send + Sync,
        M: Serialize + Send + Sync,
        C: TryInto<Coins> + Send,
        StdError: From<C::Error>,
    {
        let msg = Message::execute(contract, msg, funds)?;
        self.send_message(signer, msg, gas_opt).await
    }

    /// Send a transaction with a single [`Message::Migrate`](grug_types::Message::Migrate).
    async fn migrate<S, M>(
        &self,
        signer: &mut S,
        contract: Addr,
        new_code_hash: Hash256,
        msg: &M,
        gas_opt: GasOption,
    ) -> Result<BroadcastTxOutcome, <Self as BroadcastClient>::Error>
    where
        S: Signer + Send + Sync,
        M: Serialize + Send + Sync,
    {
        let msg = Message::migrate(contract, new_code_hash, msg)?;
        self.send_message(signer, msg, gas_opt).await
    }
}

impl<C> BroadcastClientExt for C
where
    C: BroadcastClient + SimulateClient + WithChainId + Send + Sync,
    <C as BroadcastClient>::Error:
        From<GasEstimateError> + From<StdError> + From<<C as SimulateClient>::Error>,
{
}

/// Skip the CLI prompt confirmation, always consider it as if the user accepted.
fn no_confirmation<E>(_tx: &Tx) -> Result<bool, E> {
    Ok(true)
}
