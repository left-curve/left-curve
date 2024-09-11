use {
    crate::{
        Addr, Binary, Coin, ContractInfo, GenericResult, Hash256, HashExt, InfoResponse, Json,
        JsonSerExt, NumberConst, Querier, Query, QueryResponse, StdError, StdResult, Uint256,
    },
    serde::Serialize,
    std::collections::BTreeMap,
};

/// A function that handles Wasm raw queries.
type RawQueryHandler = Box<dyn Fn(Addr, Binary) -> Option<Binary>>;

/// A function that handles Wasm smart queries.
type SmartQueryHandler = Box<dyn Fn(Addr, Json) -> GenericResult<Json>>;

/// A mock implementation of the [`Querier`](crate::Querier) trait for testing
/// purpose.
#[derive(Default)]
pub struct MockQuerier {
    info: Option<InfoResponse>,
    app_configs: BTreeMap<String, Json>,
    balances: BTreeMap<Addr, BTreeMap<String, Uint256>>,
    supplies: BTreeMap<String, Uint256>,
    codes: BTreeMap<Hash256, Binary>,
    contracts: BTreeMap<Addr, ContractInfo>,
    raw_query_handler: Option<RawQueryHandler>,
    smart_query_handler: Option<SmartQueryHandler>,
}

impl MockQuerier {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_info(mut self, info: InfoResponse) -> Self {
        self.info = Some(info);
        self
    }

    pub fn with_app_config<K, V>(mut self, key: K, value: V) -> StdResult<Self>
    where
        K: Into<String>,
        V: Serialize,
    {
        let key = key.into();
        let value = value.to_json_value()?;

        self.app_configs.insert(key, value);
        Ok(self)
    }

    pub fn with_balance<T>(mut self, address: Addr, denom: &str, amount: T) -> Self
    where
        T: Into<Uint256>,
    {
        self.balances
            .entry(address)
            .or_default()
            .insert(denom.to_string(), amount.into());
        self
    }

    pub fn with_supplies<T>(mut self, denom: &str, amount: T) -> Self
    where
        T: Into<Uint256>,
    {
        self.supplies.insert(denom.to_string(), amount.into());
        self
    }

    pub fn with_code<T>(mut self, code: T) -> Self
    where
        T: Into<Binary>,
    {
        let code = code.into();
        let code_hash = code.hash256();

        self.codes.insert(code_hash, code);
        self
    }

    pub fn with_contract(mut self, address: Addr, contract: ContractInfo) -> Self {
        self.contracts.insert(address, contract);
        self
    }

    pub fn with_raw_query_handler(mut self, handler: RawQueryHandler) -> Self {
        self.raw_query_handler = Some(handler);
        self
    }

    pub fn with_smart_query_handler(mut self, handler: SmartQueryHandler) -> Self {
        self.smart_query_handler = Some(handler);
        self
    }
}

impl Querier for MockQuerier {
    fn query_chain(&self, req: Query) -> StdResult<QueryResponse> {
        match req {
            Query::Info {} => {
                let info = self.info.clone().expect("[MockQuerier]: info is not set");
                Ok(QueryResponse::Info(info))
            },
            Query::AppConfig { key } => {
                let value = self
                    .app_configs
                    .get(&key)
                    .cloned()
                    .ok_or_else(|| StdError::data_not_found::<Json>(key.as_bytes()))?;
                Ok(QueryResponse::AppConfig(value))
            },
            Query::AppConfigs { start_after, limit } => {
                // Using the `BTreeMap::range` method is more efficient, but for
                // testing purpose this is good enough.
                let entries = self
                    .app_configs
                    .iter()
                    .filter(|(k, _)| {
                        if let Some(lower_bound) = &start_after {
                            *k > lower_bound
                        } else {
                            true
                        }
                    })
                    .take(limit.unwrap_or(u32::MAX) as usize)
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();
                Ok(QueryResponse::AppConfigs(entries))
            },
            Query::Balance { address, denom } => {
                let amount = self
                    .balances
                    .get(&address)
                    .and_then(|amounts| amounts.get(&denom))
                    .cloned()
                    .unwrap_or(Uint256::ZERO);
                Ok(QueryResponse::Balance(Coin { denom, amount }))
            },
            Query::Balances {
                address,
                start_after,
                limit,
            } => {
                let coins = self
                    .balances
                    .get(&address)
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|(denom, _)| {
                        if let Some(lower_bound) = &start_after {
                            denom > lower_bound
                        } else {
                            true
                        }
                    })
                    .take(limit.unwrap_or(u32::MAX) as usize)
                    .collect::<BTreeMap<_, _>>()
                    .try_into()?;
                Ok(QueryResponse::Balances(coins))
            },
            Query::Supply { denom } => {
                let amount = self.supplies.get(&denom).cloned().unwrap_or(Uint256::ZERO);
                Ok(QueryResponse::Supply(Coin { denom, amount }))
            },
            Query::Supplies { start_after, limit } => {
                let coins = self
                    .supplies
                    .iter()
                    .filter(|(denom, _)| {
                        if let Some(lower_bound) = &start_after {
                            *denom > lower_bound
                        } else {
                            true
                        }
                    })
                    .take(limit.unwrap_or(u32::MAX) as usize)
                    .map(|(k, v)| (k.clone(), *v))
                    .collect::<BTreeMap<_, _>>()
                    .try_into()?;
                Ok(QueryResponse::Supplies(coins))
            },
            Query::Code { hash } => {
                let code = self
                    .codes
                    .get(&hash)
                    .cloned()
                    .ok_or_else(|| StdError::data_not_found::<Binary>(hash.as_ref()))?;
                Ok(QueryResponse::Code(code))
            },
            Query::Codes { start_after, limit } => {
                let codes = self
                    .codes
                    .iter()
                    .filter(|(hash, _)| {
                        if let Some(lower_bound) = &start_after {
                            *hash > lower_bound
                        } else {
                            true
                        }
                    })
                    .take(limit.unwrap_or(u32::MAX) as usize)
                    .map(|(k, v)| (*k, v.clone()))
                    .collect();
                Ok(QueryResponse::Codes(codes))
            },
            Query::Contract { address } => {
                let contract =
                    self.contracts.get(&address).cloned().ok_or_else(|| {
                        StdError::data_not_found::<ContractInfo>(address.as_ref())
                    })?;
                Ok(QueryResponse::Contract(contract))
            },
            Query::Contracts { start_after, limit } => {
                let contracts = self
                    .contracts
                    .iter()
                    .filter(|(address, _)| {
                        if let Some(lower_bound) = &start_after {
                            *address > lower_bound
                        } else {
                            true
                        }
                    })
                    .take(limit.unwrap_or(u32::MAX) as usize)
                    .map(|(k, v)| (*k, *v))
                    .collect();
                Ok(QueryResponse::Contracts(contracts))
            },
            Query::WasmRaw { contract, key } => {
                let handler = self
                    .raw_query_handler
                    .as_ref()
                    .expect("[MockQuerier]: raw query handler not set");
                let maybe_value = handler(contract, key);
                Ok(QueryResponse::WasmRaw(maybe_value))
            },
            Query::WasmSmart { contract, msg } => {
                let handler = self
                    .smart_query_handler
                    .as_ref()
                    .expect("[MockQuerier]: smart query handler not set");
                let response = handler(contract, msg).into_std_result()?;
                Ok(QueryResponse::WasmSmart(response))
            },
            Query::Multi(requests) => {
                let responses = requests
                    .into_iter()
                    .map(|req| self.query_chain(req))
                    .collect::<StdResult<Vec<_>>>()?;
                Ok(QueryResponse::Multi(responses))
            },
        }
    }
}