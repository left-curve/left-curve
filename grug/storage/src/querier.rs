use {
    crate::{Codec, Path},
    grug_types::{Addr, Querier, QuerierExt, StdError, StdResult},
};

pub trait StorageQuerier: Querier {
    /// Query and deserialize the data corresponding to a given storage path in
    /// the given contract.
    /// Return `None` if the data is not found.
    fn may_query_wasm_path<T, C>(
        &self,
        contract: Addr,
        path: Path<'_, T, C>,
    ) -> StdResult<Option<T>>
    where
        C: Codec<T>;

    /// Query and deserialize the data corresponding to a given storage path in
    /// the given contract.
    /// Error if the data is not found.
    fn query_wasm_path<T, C>(&self, contract: Addr, path: &Path<'_, T, C>) -> StdResult<T>
    where
        C: Codec<T>;
}

impl<Q> StorageQuerier for Q
where
    Q: Querier,
{
    fn may_query_wasm_path<T, C>(
        &self,
        contract: Addr,
        path: Path<'_, T, C>,
    ) -> StdResult<Option<T>>
    where
        C: Codec<T>,
    {
        self.query_wasm_raw(contract, path.storage_key())?
            .map(|data| C::decode(&data))
            .transpose()
    }

    fn query_wasm_path<T, C>(&self, contract: Addr, path: &Path<'_, T, C>) -> StdResult<T>
    where
        C: Codec<T>,
    {
        self.query_wasm_raw(contract, path.storage_key())?
            .ok_or_else(|| StdError::data_not_found::<T>(path.storage_key()))
            .and_then(|data| C::decode(&data))
    }
}
