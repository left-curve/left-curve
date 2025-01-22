use {
    crate::{Codec, Path},
    grug_types::{Addr, Querier, QuerierExt, StdError},
};

pub trait PathQuerier: Querier {
    fn query_wasm_path<T, C>(&self, contract: Addr, path: Path<'_, T, C>) -> Result<T, Self::Error>
    where
        C: Codec<T>;
}

impl<Q> PathQuerier for Q
where
    Q: QuerierExt,
    Q::Error: From<StdError>,
{
    fn query_wasm_path<T, C>(&self, contract: Addr, path: Path<'_, T, C>) -> Result<T, Self::Error>
    where
        C: Codec<T>,
    {
        let res = self
            .query_wasm_raw(contract, path.storage_key())?
            .ok_or(StdError::data_not_found::<T>(path.storage_key()))?;

        Ok(C::decode(&res)?)
    }
}
