use {
    crate::{Codec, Path},
    grug_types::{Addr, Querier, QuerierExt, StdError},
};

pub trait StorageQuerier: Querier {
    fn query_wasm_path<T, C>(&self, contract: Addr, path: Path<'_, T, C>) -> Result<T, Self::Error>
    where
        C: Codec<T>;
}

impl<Q> StorageQuerier for Q
where
    Q: QuerierExt,
    Q::Error: From<StdError>,
{
    fn query_wasm_path<T, C>(&self, contract: Addr, path: Path<'_, T, C>) -> Result<T, Self::Error>
    where
        C: Codec<T>,
    {
        self.query_wasm_raw(contract, path.storage_key())?
            .ok_or_else(|| StdError::data_not_found::<T>(path.storage_key()))
            .and_then(|data| C::decode(&data))
            .map_err(Into::into)
    }
}
