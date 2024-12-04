use {
    crate::{Codec, IndexedMap, Item, Map, Path},
    grug_types::{Addr, Querier, Query, QueryResponse, StdError, StdResult},
};

// -------------------------------- PathQuerier --------------------------------

pub trait PathQuerier {
    fn query_wasm_path<T, C>(&self, contract: Addr, path: Path<T, C>) -> StdResult<T>
    where
        C: Codec<T>;
}

impl<Q> PathQuerier for Q
where
    Q: Querier,
{
    fn query_wasm_path<T, C>(&self, contract: Addr, path: Path<T, C>) -> StdResult<T>
    where
        C: Codec<T>,
    {
        let res = self
            .query_chain(Query::wasm_raw(contract, path.storage_key()))?
            .as_wasm_raw()
            .ok_or(StdError::data_not_found::<T>(path.storage_key()))?;

        C::decode(&res)
    }
}

// ------------------------------ QueryResponseExt -----------------------------

pub trait QueryResponseExt {
    #[allow(clippy::wrong_self_convention)]
    fn as_wasm_path<E>(self, _: E) -> Option<StdResult<E::Target>>
    where
        E: WithEncodingSchema;
}

impl QueryResponseExt for QueryResponse {
    fn as_wasm_path<E>(self, _: E) -> Option<StdResult<E::Target>>
    where
        E: WithEncodingSchema,
    {
        match self {
            QueryResponse::WasmRaw(encoded_bytes) => {
                encoded_bytes.map(|encoded_bytes| E::EncodingSchema::decode(&encoded_bytes))
            },
            _ => panic!("QueryResponse is not WasmRaw/WasmPath"),
        }
    }
}

// ----------------------------- WithEncodingSchema ----------------------------

pub trait WithEncodingSchema {
    type Target;
    type EncodingSchema: Codec<Self::Target>;
}

impl<T, C> WithEncodingSchema for Path<'_, T, C>
where
    C: Codec<T>,
{
    type EncodingSchema = C;
    type Target = T;
}

impl<T, C> WithEncodingSchema for Item<'_, T, C>
where
    C: Codec<T>,
{
    type EncodingSchema = C;
    type Target = T;
}

impl<K, T, C> WithEncodingSchema for Map<'_, K, T, C>
where
    C: Codec<T>,
{
    type EncodingSchema = C;
    type Target = T;
}

impl<K, T, I, C> WithEncodingSchema for IndexedMap<'_, K, T, I, C>
where
    C: Codec<T>,
{
    type EncodingSchema = C;
    type Target = T;
}
